use super::helpers::{fetch_driver_for_update, fetch_member_for_update, update_driver};
use super::Engine;

use async_trait::async_trait;
use sqlx::{types::Json, Acquire, Executor, Row};
use uuid::Uuid;

use crate::{api::DriverAPI, auth::User, entities::Driver, error::Error};

#[async_trait]
impl DriverAPI for Engine {
    #[tracing::instrument(skip(self))]
    async fn create_driver(&self, user: User) -> Result<Driver, Error> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;

        let member = fetch_member_for_update(&mut tx, &user.id)
            .await
            .map_err(|err| {
                if err.is_invalid_input_error() {
                    Error::unauthorized_error()
                } else {
                    err
                }
            })?;

        self.authorize(user.clone(), "create_driver", member.clone())?;

        let driver = Driver::new(user.id);

        tx.execute(
            sqlx::query("INSERT INTO drivers (id, status, data) VALUES ($1, $2, $3)")
                .bind(&driver.id)
                .bind(&driver.status.name())
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

        tx.execute(
            sqlx::query("INSERT INTO driver_priorities (driver_id, priority) VALUES ($1, 0)")
                .bind(&driver.id),
        )
        .await?;

        tx.commit().await?;

        Ok(driver)
    }

    #[tracing::instrument(skip(self))]
    async fn find_driver(&self, user: User, id: Uuid) -> Result<Driver, Error> {
        let mut conn = self.pool.acquire().await?;

        let Json(driver): Json<Driver> = conn
            .fetch_optional(sqlx::query("SELECT data FROM drivers WHERE id = $1").bind(&id))
            .await?
            .ok_or_else(|| Error::invalid_input_error())?
            .try_get("data")?;

        self.authorize(user.clone(), "read", driver.clone())?;

        Ok(driver)
    }

    #[tracing::instrument(skip(self))]
    async fn start_driver(&self, user: User, id: Uuid) -> Result<Driver, Error> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;

        let mut driver = fetch_driver_for_update(&mut tx, &id).await?;

        self.authorize(user.clone(), "start", driver.clone())?;

        driver.start()?;

        update_driver(&mut tx, &driver).await?;

        tx.commit().await?;

        Ok(driver)
    }

    #[tracing::instrument(skip(self))]
    async fn stop_driver(&self, user: User, id: Uuid) -> Result<Driver, Error> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;

        let mut driver = fetch_driver_for_update(&mut tx, &id).await?;

        self.authorize(user.clone(), "stop", driver.clone())?;

        driver.stop()?;

        update_driver(&mut tx, &driver).await?;

        tx.commit().await?;

        Ok(driver)
    }

    #[tracing::instrument(skip(self))]
    async fn update_driver_rate(
        &self,
        user: User,
        id: Uuid,
        min_fare: f64,
        rate: f64,
    ) -> Result<(), Error> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;

        let driver = fetch_driver_for_update(&mut tx, &user.id).await?;

        self.authorize(user.clone(), "update_rate", driver.clone())?;

        tx.execute(
            sqlx::query("UPDATE driver_rates SET min_fare = $2, rate = $3 WHERE driver_id = $1")
                .bind(&id)
                .bind(min_fare)
                .bind(rate),
        )
        .await?;

        tx.commit().await?;

        Ok(())
    }
}
