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

    async fn exec_tx<F, S, E>(&self, f: F) -> Result<S, TransactionError<E>>
    where
        for<'tx> F: FnOnce(
            &'tx mut sqlx::Transaction<Self::DB>,
        ) -> Pin<Box<dyn Future<Output = Result<S, E>> + 'tx>>;
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

#[async_trait(?Send)]
impl DBHandle for PgStore {
    type DB = Postgres;

    async fn exec_tx<F, S, E>(&self, f: F) -> Result<S, TransactionError<E>>
    where
        for<'tx> F: FnOnce(
            &'tx mut sqlx::Transaction<Self::DB>,
        ) -> Pin<Box<dyn Future<Output = Result<S, E>> + 'tx>>,
    {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| TransactionError::DBError(e))?;

        let result = f(&mut tx).await;

        match result {
            Ok(r) => {
                tx.commit()
                    .await
                    .map_err(|e| TransactionError::DBError(e))?;
                Ok(r)
            }
            Err(e) => match tx.rollback().await {
                Ok(_) => Err(TransactionError::ApplicationError(e)),
                Err(e) => Err(TransactionError::DBError(e)),
            },
        }
    }

    async fn acquire_conn(&self) -> Result<PoolConnection<Self::DB>, sqlx::Error> {
        self.acquire_conn().await
    }
}
