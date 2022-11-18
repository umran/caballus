use async_trait::async_trait;
use futures::TryStreamExt;
use sqlx::{types::Json, Acquire, Executor, Pool, Postgres, Row};
use uuid::Uuid;

use crate::{
    api::{RouteAPI, TripAPI, API},
    entities::{Bid, Driver, Place, Route, Trip},
    error::{self, invalid_input_error, Error},
};

type Database = Postgres;

#[derive(Debug)]
pub struct Engine {
    pool: Pool<Database>,
}

impl Engine {
    pub async fn new(pool: Pool<Database>) -> Result<Self, Error> {
        pool.execute("CREATE TABLE IF NOT EXISTS routes (id UUID PRIMARY KEY, data jsonb)")
            .await?;
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
impl RouteAPI for Engine {
    async fn create_route(&self, origin: Place, destination: Place) -> Result<Route, Error> {
        Ok(Route::new(origin, destination))
    }

    async fn find_route(&self, id: Uuid) -> Result<Route, Error> {
        let mut conn = self.pool.acquire().await?;

        let maybe_result = conn
            .fetch_optional(sqlx::query("SELECT data FROM routes WHERE id = $1").bind(&id))
            .await?;

        if let Some(result) = maybe_result {
            let Json(route) = result.try_get("data")?;

            return Ok(route);
        }

        Err(error::invalid_input_error())
    }
}

#[async_trait]
impl TripAPI for Engine {
    async fn find_trip(&self, id: Uuid) -> Result<Trip, Error> {
        let mut conn = self.pool.acquire().await?;

        let maybe_result = conn
            .fetch_optional(sqlx::query("SELECT data FROM trips WHERE id = $1").bind(&id))
            .await?;

        if let Some(result) = maybe_result {
            let Json(trip) = result.try_get("data")?;
            return Ok(trip);
        }

        Err(error::invalid_input_error())
    }

    async fn create_trip(&self, route_id: Uuid, passenger_id: Uuid) -> Result<Trip, Error> {
        let mut conn = self.pool.acquire().await?;

        let maybe_result = conn
            .fetch_optional(sqlx::query("SELECT id FROM routes WHERE id = $1").bind(&route_id))
            .await?;

        if let Some(_) = maybe_result {
            let trip = Trip::new(route_id, passenger_id);

            conn.execute(
                sqlx::query("INSERT INTO trips (id, status, data) VALUES ($1, $2, $3)")
                    .bind(&trip.id)
                    .bind(&trip.status_string())
                    .bind(Json(&trip)),
            )
            .await?;

            return Ok(trip);
        }

        Err(error::invalid_input_error())
    }

    async fn expand_search(&self, id: Uuid) -> Result<Trip, Error> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;

        let maybe_result = tx
            .fetch_optional(
                sqlx::query("SELECT data FROM trips WHERE id = $1 FOR UPDATE").bind(&id),
            )
            .await?;

        if let Some(result) = maybe_result {
            let Json::<Trip>(mut trip) = result.try_get("data")?;
            trip.expand_search()?;

            tx.execute(
                sqlx::query("UPDATE trips SET data = $2 WHERE id = $1")
                    .bind(&id)
                    .bind(Json(&trip)),
            )
            .await?;

            tx.commit().await?;

            return Ok(trip);
        }

        Err(invalid_input_error())
    }

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

            let driver_result = tx
                .fetch_one(
                    sqlx::query("SELECT data FROM drivers WHERE id = $1 FOR UPDATE")
                        .bind(&driver_id),
                )
                .await?;
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

    async fn submit_bid(&self, bid: Bid) -> Result<(), Error> {
        Err(Error {
            code: 0,
            message: "unimplemented".to_string(),
        })
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
