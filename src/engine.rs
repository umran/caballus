use async_trait::async_trait;
use chrono::Utc;
use futures::TryStreamExt;
use serde_json::json;
use sqlx::{types::Json, Acquire, Executor, Pool, Postgres, Row};
use uuid::Uuid;

use crate::{
    api::{LocationAPI, RouteAPI, TripAPI, API},
    entities::{Bid, Driver, Location, LocationSource, Route, Trip},
    error::{invalid_input_error, Error},
    external::google_maps,
};

type Database = Postgres;

#[derive(Debug)]
pub struct Engine {
    pool: Pool<Database>,
}

impl Engine {
    #[tracing::instrument]
    pub async fn new(pool: Pool<Database>) -> Result<Self, Error> {
        // location service
        pool.execute("CREATE TABLE IF NOT EXISTS locations (token UUID PRIMARY KEY, data jsonb)")
            .await?;

        // route service
        pool.execute("CREATE TABLE IF NOT EXISTS routes (token UUID PRIMARY KEY, data jsonb)")
            .await?;

        // trip service
        pool.execute("CREATE TABLE IF NOT EXISTS trips (id UUID PRIMARY KEY, status VARCHAR NOT NULL, data jsonb)")
            .await?;
        pool.execute("CREATE TABLE IF NOT EXISTS drivers (id UUID PRIMARY KEY, status VARCHAR NOT NULL, data jsonb)")
            .await?;
        pool.execute("CREATE TABLE IF NOT EXISTS bids (id UUID PRIMARY KEY, trip_id UUID NOT NULL, driver_id UUID NOT NULL, amount INT4 NOT NULL, CONSTRAINT fk_bid_trip FOREIGN KEY(trip_id) REFERENCES trips(id), CONSTRAINT fk_bid_driver FOREIGN KEY(driver_id) REFERENCES drivers(id))")
            .await?;

        Ok(Self { pool })
    }
}

#[async_trait]
impl LocationAPI for Engine {
    #[tracing::instrument]
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

    #[tracing::instrument]
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
    #[tracing::instrument]
    async fn create_route(
        &self,
        origin_token: Uuid,
        destination_token: Uuid,
    ) -> Result<Route, Error> {
        let origin = self.find_location(origin_token).await?;
        let destination = self.find_location(destination_token).await?;

        let route = Route::new(origin, destination, json!(""));

        let mut conn = self.pool.acquire().await?;
        conn.execute(
            sqlx::query("INSERT INTO routes (token, data) VALUES ($1, $2)")
                .bind(&route.token)
                .bind(Json(&route)),
        )
        .await?;

        Ok(route)
    }

    #[tracing::instrument]
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
impl TripAPI for Engine {
    #[tracing::instrument]
    async fn find_trip(&self, id: Uuid) -> Result<Trip, Error> {
        let mut conn = self.pool.acquire().await?;

        let maybe_result = conn
            .fetch_optional(sqlx::query("SELECT data FROM trips WHERE id = $1").bind(&id))
            .await?;

        let result = maybe_result.ok_or_else(|| invalid_input_error())?;
        let Json(trip) = result.try_get("data")?;

        Ok(trip)
    }

    #[tracing::instrument]
    async fn create_trip(&self, route_token: Uuid, passenger_id: Uuid) -> Result<Trip, Error> {
        let route = self.find_route(route_token).await?;
        let trip = Trip::new(passenger_id, route);

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

    #[tracing::instrument]
    async fn expand_search(&self, id: Uuid) -> Result<Trip, Error> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;

        let maybe_result = tx
            .fetch_optional(
                sqlx::query("SELECT data FROM trips WHERE id = $1 FOR UPDATE").bind(&id),
            )
            .await?;

        let result = maybe_result.ok_or_else(|| invalid_input_error())?;

        let Json::<Trip>(mut trip) = result.try_get("data")?;
        trip.expand_search()?;

        tx.execute(
            sqlx::query("UPDATE trips SET data = $2 WHERE id = $1")
                .bind(&id)
                .bind(Json(&trip)),
        )
        .await?;

        tx.commit().await?;

        Ok(trip)
    }

    #[tracing::instrument]
    async fn evaluate_bids(&self, id: Uuid) -> Result<Option<Trip>, Error> {
        let mut conn = self.pool.acquire().await?;

        let mut results = conn.fetch(
            sqlx::query("SELECT id, driver_id, fare FROM bids WHERE trip_id = $1").bind(&id),
        );

        while let Some(row) = results.try_next().await? {
            let trip_id = id.clone();
            let bid_id: Uuid = row.try_get("id")?;
            let driver_id: Uuid = row.try_get("driver_id")?;

            let mut conn = self.pool.acquire().await?;
            let mut tx = conn.begin().await?;

            let maybe_driver_result = tx
                .fetch_optional(
                    sqlx::query("SELECT data FROM drivers WHERE id = $1 FOR UPDATE")
                        .bind(&driver_id),
                )
                .await?;

            if maybe_driver_result.is_none() {
                continue;
            }

            let driver_result = maybe_driver_result.unwrap();
            let Json::<Driver>(mut driver) = driver_result.try_get("data")?;

            if !driver.is_available() {
                continue;
            }

            driver.assign_trip(trip_id.clone())?;

            tx.execute(
                sqlx::query("UPDATE drivers SET status = $2, data = $3 WHERE id = $1")
                    .bind(&driver_id)
                    .bind(&driver.status_string())
                    .bind(Json(&driver)),
            )
            .await?;

            let trip_result = tx
                .fetch_one(
                    sqlx::query("SELECT data FROM trips WHERE id = $1 FOR UPDATE").bind(&trip_id),
                )
                .await?;
            let Json::<Trip>(mut trip) = trip_result.try_get("data")?;

            trip.select_bid(bid_id)?;

            tx.execute(
                sqlx::query("UPDATE trips SET status = $2, data = $3 WHERE id = $1")
                    .bind(&trip_id)
                    .bind(&trip.status_string())
                    .bind(Json(&trip)),
            )
            .await?;

            tx.commit().await?;

            return Ok(Some(trip));
        }

        Ok(None)
    }

    #[tracing::instrument]
    async fn submit_bid(&self, trip_id: Uuid, driver_id: Uuid, amount: i64) -> Result<Bid, Error> {
        let mut conn = self.pool.acquire().await?;

        // perform a weak (non-consistent) check to ensure bid submitted before deadline
        let maybe_result = conn
            .fetch_optional(sqlx::query("SELECT data FROM trips WHERE id = $1").bind(&trip_id))
            .await?;

        let result = maybe_result.ok_or_else(|| invalid_input_error())?;
        let Json(trip): Json<Trip> = result.try_get("data")?;

        if Utc::now() > trip.search_deadline()? {
            return Err(invalid_input_error());
        }

        // make sure driver exists and is AVAILABLE while bid is created
        let mut tx = conn.begin().await?;

        let maybe_driver_result = tx
            .fetch_optional(
                sqlx::query("SELECT data FROM drivers WHERE id = $1 FOR UPDATE").bind(&driver_id),
            )
            .await?;
        let driver_result = maybe_driver_result.ok_or_else(|| invalid_input_error())?;
        let Json(driver): Json<Driver> = driver_result.try_get("data")?;

        if !driver.is_available() {
            return Err(invalid_input_error());
        }

        let bid = Bid::new(trip_id, driver_id, amount);

        tx.execute(
            sqlx::query(
                "INSERT INTO bids (id, trip_id, driver_id, amount) VALUES ($1, $2, $3, $4)",
            )
            .bind(&bid.id)
            .bind(&bid.trip_id)
            .bind(&bid.driver_id)
            .bind(&bid.amount),
        )
        .await?;

        tx.commit().await?;

        Ok(bid)
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
