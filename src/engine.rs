use async_trait::async_trait;
use chrono::{Duration, Utc};
use geo_types::Geometry;
use geozero::wkb;
use serde_json::json;
use sqlx::{types::Json, Acquire, Executor, Pool, Postgres, Row};
use uuid::Uuid;

use crate::{
    api::{DriverAPI, LocationAPI, QuoteAPI, RouteAPI, TripAPI, API},
    entities::{Coordinates, Driver, Location, LocationSource, Quote, Route, Trip},
    error::{invalid_input_error, Error},
    external::google_maps,
};

type Database = Postgres;

#[derive(Debug)]
pub struct Engine {
    pool: Pool<Database>,
}

impl Engine {
    #[tracing::instrument(name = "Engine::new", skip_all)]
    pub async fn new(pool: Pool<Database>) -> Result<Self, Error> {
        // location service (KV store)
        pool.execute("DROP TABLE IF EXISTS locations CASCADE")
            .await?;
        pool.execute("CREATE TABLE locations (token UUID PRIMARY KEY, data JSONB NOT NULL)")
            .await?;

        // route service (KV store)
        pool.execute("DROP TABLE IF EXISTS routes CASCADE").await?;
        pool.execute("CREATE TABLE routes (token UUID PRIMARY KEY, data JSONB NOT NULL)")
            .await?;

        // quote service (KV store)
        pool.execute("DROP TABLE IF EXISTS quotes CASCADE").await?;
        pool.execute("CREATE TABLE quotes (token UUID PRIMARY KEY, data JSONB NOT NULL)")
            .await?;

        // trip service
        pool.execute("DROP TABLE IF EXISTS trips CASCADE").await?;
        pool.execute("CREATE TABLE trips (id UUID PRIMARY KEY, status VARCHAR NOT NULL, data JSONB NOT NULL)")
            .await?;

        pool.execute("DROP TABLE IF EXISTS drivers CASCADE").await?;
        pool.execute("CREATE TABLE drivers (id UUID PRIMARY KEY, user_id UUID NOT NULL UNIQUE, status VARCHAR NOT NULL, data JSONB NOT NULL)")
            .await?;

        pool.execute("DROP TABLE IF EXISTS driver_rates CASCADE")
            .await?;
        pool.execute(
            "CREATE TABLE driver_rates (driver_id UUID PRIMARY KEY, min_fare DECIMAL, rate DECIMAL)",
        )
        .await?;

        pool.execute("DROP TABLE IF EXISTS driver_locations CASCADE")
            .await?;
        pool.execute("CREATE TABLE driver_locations (driver_id UUID PRIMARY KEY, location geometry(Point), expiry TIMESTAMP)")
            .await?;

        Ok(Self { pool })
    }
}

#[async_trait]
impl LocationAPI for Engine {
    #[tracing::instrument(skip(self))]
    async fn create_location(&self, source: LocationSource) -> Result<Location, Error> {
        let location: Location = match source {
            LocationSource::Coordinates(coordinates) => Location::new(coordinates, "".into()),
            LocationSource::GooglePlaces {
                place_id,
                session_token,
            } => {
                let place = google_maps::find_place(place_id, session_token).await?;
                Location::new(place.geometry.location, place.formatted_address)
            }
        };

        let mut conn = self.pool.acquire().await?;

        conn.execute(
            sqlx::query("INSERT INTO locations (token, data) VALUES ($1, $2)")
                .bind(&location.token)
                .bind(Json(&location)),
        )
        .await?;

        Ok(location)
    }

    #[tracing::instrument(skip(self))]
    async fn find_location(&self, token: Uuid) -> Result<Location, Error> {
        let mut conn = self.pool.acquire().await?;

        let maybe_result = conn
            .fetch_optional(sqlx::query("SELECT data FROM locations WHERE token = $1").bind(&token))
            .await?;

        let result = maybe_result.ok_or_else(|| invalid_input_error())?;
        let Json(location) = result.try_get("data")?;

        Ok(location)
    }
}

#[async_trait]
impl RouteAPI for Engine {
    #[tracing::instrument(skip(self))]
    async fn create_route(
        &self,
        origin_token: Uuid,
        destination_token: Uuid,
    ) -> Result<Route, Error> {
        let origin = self.find_location(origin_token).await?;
        let destination = self.find_location(destination_token).await?;

        let route = Route::new(origin, destination, json!(""), 4500.0);

        let mut conn = self.pool.acquire().await?;
        conn.execute(
            sqlx::query("INSERT INTO routes (token, data) VALUES ($1, $2)")
                .bind(&route.token)
                .bind(Json(&route)),
        )
        .await?;

        Ok(route)
    }

    #[tracing::instrument(skip(self))]
    async fn find_route(&self, token: Uuid) -> Result<Route, Error> {
        let mut conn = self.pool.acquire().await?;

        let maybe_result = conn
            .fetch_optional(sqlx::query("SELECT data FROM routes WHERE token = $1").bind(&token))
            .await?;

        let result = maybe_result.ok_or_else(|| invalid_input_error())?;
        let Json(route) = result.try_get("data")?;

        Ok(route)
    }
}

#[async_trait]
impl QuoteAPI for Engine {
    #[tracing::instrument(skip(self))]
    async fn create_quote(&self, route_token: Uuid) -> Result<Option<Quote>, Error> {
        let route = self.find_route(route_token).await?;

        let origin_location: Geometry<f64> = route.origin.coordinates.clone().into();
        let search_radius = 2000.0;

        let query = "
            SELECT
                percentile_cont(0.75) WITHIN GROUP (
                    ORDER BY
                        fares.fare ASC
                ) AS max_fare
            FROM
                (
                    SELECT
                        GREATEST(
                            r.min_fare, r.rate * (
                                ST_Distance(l.location, ST_SetSRID($1, 4326)) + $2
                            )
                        ) AS fare
                    FROM
                        drivers d
                        LEFT JOIN driver_rates r ON d.id = r.driver_id
                        LEFT JOIN driver_locations l ON d.id = l.driver_id
                    WHERE
                        d.status = 'AVAILABLE'
                        AND r.rate IS NOT NULL
                        AND l.location IS NOT NULL
                        AND l.expiry > now()
                        AND ST_DWithin(l.location, ST_SetSRID($1, 4326), $3)
                ) AS fares
        ";

        let mut conn = self.pool.acquire().await?;

        let maybe_max_fare: Option<f64> = conn
            .fetch_one(
                sqlx::query(query)
                    .bind(wkb::Encode(origin_location))
                    .bind(route.distance)
                    .bind(search_radius),
            )
            .await?
            .try_get("max_fare")?;

        match maybe_max_fare {
            Some(max_fare) => {
                let quote = Quote::new(route, max_fare);

                conn.execute(
                    sqlx::query("INSERT INTO quotes (token, data) VALUES ($1, $2)")
                        .bind(&quote.token)
                        .bind(Json(&quote)),
                )
                .await?;

                Ok(Some(quote))
            }
            None => Ok(None),
        }
    }

    #[tracing::instrument(skip(self))]
    async fn find_quote(&self, quote_token: Uuid) -> Result<Quote, Error> {
        let mut conn = self.pool.acquire().await?;

        let maybe_result = conn
            .fetch_optional(
                sqlx::query("SELECT data FROM quotes WHERE token = $1").bind(&quote_token),
            )
            .await?;

        let result = maybe_result.ok_or_else(|| invalid_input_error())?;
        let Json(quote) = result.try_get("data")?;

        Ok(quote)
    }
}

#[async_trait]
impl TripAPI for Engine {
    #[tracing::instrument(skip(self))]
    async fn find_trip(&self, id: Uuid) -> Result<Trip, Error> {
        let mut conn = self.pool.acquire().await?;

        let maybe_result = conn
            .fetch_optional(sqlx::query("SELECT data FROM trips WHERE id = $1").bind(&id))
            .await?;

        let result = maybe_result.ok_or_else(|| invalid_input_error())?;
        let Json(trip) = result.try_get("data")?;

        Ok(trip)
    }

    #[tracing::instrument(skip(self))]
    async fn create_trip(&self, quote_token: Uuid, passenger_id: Uuid) -> Result<Trip, Error> {
        let quote = self.find_quote(quote_token).await?;
        let trip = Trip::new(passenger_id, quote.route, quote.max_fare);

        let mut conn = self.pool.acquire().await?;

        // TODO: ensure passenger exists and does not have another active trip while trip is created

        conn.execute(
            sqlx::query("INSERT INTO trips (id, status, data) VALUES ($1, $2, $3)")
                .bind(&trip.id)
                .bind(&trip.status_string())
                .bind(Json(&trip)),
        )
        .await?;

        Ok(trip)
    }

    #[tracing::instrument(skip(self))]
    async fn request_driver(&self, id: Uuid) -> Result<Trip, Error> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;

        // fetch trip for update
        let Json(mut trip): Json<Trip> = tx
            .fetch_optional(
                sqlx::query("SELECT data FROM trips WHERE id = $1 FOR UPDATE").bind(&id),
            )
            .await?
            .ok_or_else(|| invalid_input_error())?
            .try_get("data")?;

        let origin_location: Geometry<f64> = trip.route.origin.coordinates.clone().into();
        let trip_distance = trip.route.distance;
        let search_radius = 2000.0;

        // fetch potential driver ids
        let query = "
            SELECT
                d.id AS driver_id
            FROM
                drivers d
                LEFT JOIN driver_rates r ON d.id = r.driver_id
                LEFT JOIN driver_locations l ON d.id = l.driver_id
            WHERE
                d.status = 'AVAILABLE'
                AND r.rate IS NOT NULL
                AND l.location IS NOT NULL
                AND l.expiry > now()
                AND ST_DWithin(l.location, ST_SetSRID($1, 4326), $3)
                AND
                    GREATEST(
                        r.min_fare, r.rate * (
                            ST_Distance(l.location, ST_SetSRID($1, 4326)) + $2
                        )
                    ) <= $4
            ORDER BY
                ST_Distance(l.location, ST_SetSRID($1, 4326)) ASC
        ";

        let results = tx
            .fetch_all(
                sqlx::query(query)
                    .bind(wkb::Encode(origin_location.clone()))
                    .bind(trip_distance)
                    .bind(search_radius)
                    .bind(trip.max_fare),
            )
            .await?;

        for result in results.iter() {
            let driver_id: Uuid = result.try_get("driver_id")?;

            // fetch driver for update
            let query = "
                SELECT
                    d.data AS driver,
                    GREATEST(
                        r.min_fare, r.rate * (
                            ST_Distance(l.location, ST_SetSRID($2, 4326)) + $3
                        )
                    ) AS fare
                FROM
                    drivers d
                    LEFT JOIN driver_rates r ON d.id = r.driver_id
                    LEFT JOIN driver_locations l ON d.id = l.driver_id
                WHERE
                    d.id = $1
                    AND d.status = 'AVAILABLE'
                    AND r.rate IS NOT NULL
                    AND l.location IS NOT NULL
                    AND l.expiry > now()
                    AND ST_DWithin(l.location, ST_SetSRID($2, 4326), $4)
                    AND
                        GREATEST(
                            r.min_fare, r.rate * (
                                ST_Distance(l.location, ST_SetSRID($2, 4326)) + $3
                            )
                        ) <= $5
                FOR UPDATE
            ";

            let maybe_result = tx
                .fetch_optional(
                    sqlx::query(query)
                        .bind(&driver_id)
                        .bind(wkb::Encode(origin_location.clone()))
                        .bind(trip_distance)
                        .bind(search_radius)
                        .bind(trip.max_fare),
                )
                .await?;

            if maybe_result.is_none() {
                continue;
            }

            // note that unwrap will never panic because it is never called if maybe_result is none
            let result = maybe_result.unwrap();

            let Json(mut driver): Json<Driver> = result.try_get("driver")?;
            let fare: f64 = result.try_get("fare")?;

            driver.request(trip.id.clone())?;

            tx.execute(
                sqlx::query("UPDATE drivers SET status = $2, data = $3 WHERE id = $1")
                    .bind(&driver.id)
                    .bind(driver.status_string())
                    .bind(Json(&driver)),
            )
            .await?;

            trip.request_driver(driver_id, fare)?;

            tx.execute(
                sqlx::query("UPDATE trips SET status = $2, data = $3 WHERE id = $1")
                    .bind(&trip.id)
                    .bind(trip.status_string())
                    .bind(Json(&trip)),
            )
            .await?;

            tx.commit().await?;
            return Ok(trip);
        }

        Ok(trip)
    }
}

#[async_trait]
impl DriverAPI for Engine {
    #[tracing::instrument(skip(self))]
    async fn create_driver(&self, user_id: Uuid) -> Result<Driver, Error> {
        let driver = Driver::new(user_id);

        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;

        let existing_record = tx
            .fetch_optional(
                sqlx::query("SELECT id FROM drivers WHERE user_id = $1 FOR UPDATE")
                    .bind(&driver.user_id),
            )
            .await?;

        if existing_record.is_some() {
            tracing::info!("driver already exists for user_id: {:?}", &driver.user_id);
            return Err(invalid_input_error());
        }

        tx.execute(
            sqlx::query("INSERT INTO drivers (id, user_id, status, data) VALUES ($1, $2, $3, $4)")
                .bind(&driver.id)
                .bind(&driver.user_id)
                .bind(&driver.status_string())
                .bind(Json(&driver)),
        )
        .await?;

        tx.execute(
            sqlx::query(
                "INSERT INTO driver_rates (driver_id, min_fare, rate) VALUES ($1, NULL, NULL)",
            )
            .bind(&driver.id),
        )
        .await?;

        tx.execute(sqlx::query(
            "INSERT INTO driver_locations (driver_id, location, expiry) VALUES ($1, NULL, NULL)",
        ).bind(&driver.id))
        .await?;

        tx.commit().await?;

        Ok(driver)
    }

    #[tracing::instrument(skip(self))]
    async fn find_driver(&self, id: Uuid) -> Result<Driver, Error> {
        let mut conn = self.pool.acquire().await?;

        let Json(driver) = conn
            .fetch_optional(sqlx::query("SELECT data FROM drivers WHERE id = $1").bind(&id))
            .await?
            .ok_or_else(|| invalid_input_error())?
            .try_get("data")?;

        Ok(driver)
    }

    #[tracing::instrument(skip(self))]
    async fn start_driver(&self, id: Uuid) -> Result<Driver, Error> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;

        let Json(mut driver): Json<Driver> = tx
            .fetch_optional(
                sqlx::query("SELECT data FROM drivers WHERE id = $1 FOR UPDATE").bind(&id),
            )
            .await?
            .ok_or_else(|| invalid_input_error())?
            .try_get("data")?;

        driver.start()?;

        tx.execute(
            sqlx::query("UPDATE drivers SET status = $2, data = $3 WHERE id = $1")
                .bind(&id)
                .bind(driver.status_string())
                .bind(Json(&driver)),
        )
        .await?;

        tx.commit().await?;

        Ok(driver)
    }

    #[tracing::instrument(skip(self))]
    async fn stop_driver(&self, id: Uuid) -> Result<Driver, Error> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;

        let Json(mut driver): Json<Driver> = tx
            .fetch_optional(
                sqlx::query("SELECT data FROM drivers WHERE id = $1 FOR UPDATE").bind(&id),
            )
            .await?
            .ok_or_else(|| invalid_input_error())?
            .try_get("data")?;

        driver.stop()?;

        tx.execute(
            sqlx::query("UPDATE drivers SET status = $2, data = $3 WHERE id = $1")
                .bind(&id)
                .bind(driver.status_string())
                .bind(Json(&driver)),
        )
        .await?;

        tx.commit().await?;

        Ok(driver)
    }

    #[tracing::instrument(skip(self))]
    async fn update_driver_location(&self, id: Uuid, location: Coordinates) -> Result<(), Error> {
        let mut conn = self.pool.acquire().await?;

        let location: Geometry<f64> = location.into();

        conn.execute(
            sqlx::query(
                "UPDATE driver_locations SET location = ST_SetSRID($2, 4326), expiry = $3 WHERE driver_id = $1",
            )
            .bind(&id)
            .bind(wkb::Encode(location))
            .bind(Utc::now() + Duration::seconds(60)),
        )
        .await?;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn update_driver_rate(&self, id: Uuid, min_fare: f64, rate: f64) -> Result<(), Error> {
        let mut conn = self.pool.acquire().await?;

        conn.execute(
            sqlx::query("UPDATE driver_rates SET min_fare = $2, rate = $3 WHERE driver_id = $1")
                .bind(&id)
                .bind(min_fare)
                .bind(rate),
        )
        .await?;

        Ok(())
    }
}

impl API for Engine {}

#[test]
fn new_engine() {
    use crate::db::PgPool;
    use tokio_test::block_on;

    let PgPool(pool) = block_on(PgPool::new(
        "postgresql://caballus:caballus@localhost:5432/caballus",
        5,
    ))
    .unwrap();

    block_on(Engine::new(pool)).unwrap();
}
