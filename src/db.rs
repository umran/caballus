use async_trait::async_trait;
use core::future::Future;
use sqlx::{pool::PoolConnection, postgres::PgPoolOptions, Database, Executor, Pool, Postgres};
use std::pin::Pin;

#[derive(Debug)]
pub enum TransactionError<E> {
    ApplicationError(E),
    DBError(sqlx::Error),
}

#[async_trait(?Send)]
pub trait DBHandle {
    type DB: Database;

    async fn exec_tx<E, F>(&self, f: F) -> Result<(), TransactionError<E>>
    where
        for<'tx> F: FnOnce(
            &'tx mut sqlx::Transaction<Self::DB>,
        ) -> Pin<Box<dyn Future<Output = Result<(), E>> + 'tx>>;
    async fn acquire_conn(&self) -> Result<PoolConnection<Self::DB>, sqlx::Error>;
}

pub struct PgStore {
    pool: Pool<Postgres>,
}

impl PgStore {
    pub async fn new(db_uri: &str, max_connections: u32) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .connect(db_uri)
            .await?;

        // TODO: move this to migrations
        pool.execute("CREATE TABLE IF NOT EXISTS routes (id SERIAL PRIMARY KEY, data jsonb)")
            .await?;
        pool.execute("CREATE TABLE IF NOT EXISTS trips (id SERIAL PRIMARY KEY, status VARCHAR NOT NULL, data jsonb)")
            .await?;
        pool.execute("CREATE TABLE IF NOT EXISTS drivers (id SERIAL PRIMARY KEY, status VARCHAR NOT NULL, data jsonb)")
            .await?;
        pool.execute("CREATE TABLE IF NOT EXISTS bids (id SERIAL PRIMARY KEY, trip_id INT4 NOT NULL, driver_id INT4 NOT NULL, fare INT4 NOT NULL, CONSTRAINT fk_bid_trip FOREIGN KEY(trip_id) REFERENCES trips(id), CONSTRAINT fk_bid_driver FOREIGN KEY(driver_id) REFERENCES drivers(id))")
            .await?;

        Ok(Self { pool })
    }
}

#[async_trait(?Send)]
impl DBHandle for PgStore {
    type DB = Postgres;

    async fn exec_tx<E, F>(&self, f: F) -> Result<(), TransactionError<E>>
    where
        for<'tx> F: FnOnce(
            &'tx mut sqlx::Transaction<Self::DB>,
        ) -> Pin<Box<dyn Future<Output = Result<(), E>> + 'tx>>,
    {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| TransactionError::DBError(e))?;

        if let Err(e) = f(&mut tx).await {
            tx.rollback()
                .await
                .map_err(|e| TransactionError::DBError(e))?;
            return Err(TransactionError::ApplicationError(e));
        }

        tx.commit().await.map_err(|e| TransactionError::DBError(e))
    }

    async fn acquire_conn(&self) -> Result<PoolConnection<Self::DB>, sqlx::Error> {
        self.acquire_conn().await
    }
}
