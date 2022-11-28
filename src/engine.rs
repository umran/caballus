use async_trait::async_trait;
use chrono::Utc;
use futures::TryStreamExt;
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
        pool.execute("CREATE TABLE drivers (id UUID PRIMARY KEY, status VARCHAR NOT NULL, data JSONB NOT NULL)")
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

        let route = Route::new(origin, destination, json!(""), 1000.0);

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
    async fn create_quote(&self, route_token: Uuid) -> Result<Quote, Error> {
        let route = self.find_route(route_token).await?;

        let origin_location: Geometry<f64> = route.origin.coordinates.clone().into();

        let query = "
            SELECT
                percentile_cont(0.75) WITHIN GROUP (
                    ORDER BY
                        fares.amount ASC
                ) AS percentile_75
            FROM
                (
                    SELECT
                        GREATEST(
                            r.min_fare, r.rate * (
                                ST_Distance(l.location, ST_SetSRID($1, 4326)) + $3
                            )
                        ) AS amount
                    FROM
                        drivers d
                        LEFT JOIN driver_rates r ON d.id = r.driver_id
                        LEFT JOIN driver_locations l ON d.id = l.driver_id
                    WHERE
                        d.status = 'AVAILABLE'
                        AND r.rate IS NOT NULL
                        AND l.location IS NOT NULL
                        AND l.expiry > now()
                        AND ST_DWithin(l.location, ST_SetSRID($1, 4326), $2)
                ) AS fares
        ";

        // return a dummy quote for now
        Ok(Quote::new(route, 50.0))
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
        let trip = Trip::new(passenger_id, quote.route, quote.amount);

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
    async fn assign_driver(&self, id: Uuid) -> Result<Trip, Error> {
        unimplemented!()
    }
}

#[async_trait]
impl DriverAPI for Engine {
    #[tracing::instrument(skip(self))]
    async fn create_driver(&self, user_id: Uuid) -> Result<Driver, Error> {
        unimplemented!()
    }

    #[tracing::instrument(skip(self))]
    async fn find_driver(&self, id: Uuid) -> Result<Driver, Error> {
        unimplemented!()
    }

    #[tracing::instrument(skip(self))]
    async fn report_location(&self, id: Uuid, location: Coordinates) -> Result<(), Error> {
        let mut conn = self.pool.acquire().await?;

        let location: Geometry<f64> = location.into();

        conn.execute(
            sqlx::query(
                "UPDATE driver_locations SET location = ST_SetSRID($2, 4326) WHERE driver_id = $1",
            )
            .bind(&id)
            .bind(&wkb::Encode(location)),
        )
        .await?;

        unimplemented!()
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
