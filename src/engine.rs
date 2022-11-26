use async_trait::async_trait;
use chrono::Utc;
use futures::TryStreamExt;
use serde_json::json;
use sqlx::{types::Json, Acquire, Executor, Pool, Postgres, Row};
use uuid::Uuid;

use crate::{
    api::{LocationAPI, QuoteAPI, RouteAPI, TripAPI, API},
    entities::{Location, LocationSource, Quote, Route, Trip},
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
        pool.execute("CREATE TABLE IF NOT EXISTS locations (token UUID PRIMARY KEY, data JSONB)")
            .await?;

        // route service (KV store)
        pool.execute("CREATE TABLE IF NOT EXISTS routes (token UUID PRIMARY KEY, data JSONB)")
            .await?;

        // quote service (KV store)
        pool.execute("CREATE TABLE IF NOT EXISTS quotes (token UUID PRIMARY KEY, data JSONB)")
            .await?;

        // trip service
        pool.execute("CREATE TABLE IF NOT EXISTS trips (id UUID PRIMARY KEY, status VARCHAR NOT NULL, data JSONB)")
            .await?;
        pool.execute("CREATE TABLE IF NOT EXISTS drivers (id UUID PRIMARY KEY, status VARCHAR NOT NULL, data JSONB)")
            .await?;
        pool.execute(
            "CREATE TABLE IF NOT EXISTS driver_rates (driver_id UUID PRIMARY KEY, data JSONB)",
        )
        .await?;
        pool.execute("CREATE TABLE IF NOT EXISTS driver_locations (driver_id UUID PRIMARY KEY, location geometry(Point), expiry TIMESTAMP)")
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

        // return a dummy quote for now
        Ok(Quote::new(route, 50.0))
    }

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
