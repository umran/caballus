use super::helpers::{fetch_driver_for_update, update_driver};
use super::Engine;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use geo_types::Geometry;
use geozero::wkb;
use sqlx::{types::Json, Acquire, Executor, Row};
use uuid::Uuid;

use crate::{
    api::{DriverAPI, API},
    auth::User,
    entities::{Coordinates, Driver},
    error::{invalid_input_error, Error},
};

#[async_trait]
impl DriverAPI for Engine {
    #[tracing::instrument(skip(self))]
    async fn create_driver(&self, user: User) -> Result<Driver, Error> {
        let driver = Driver::new(user.id);

        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;

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

        let Json(driver) = conn
            .fetch_optional(sqlx::query("SELECT data FROM drivers WHERE id = $1").bind(&id))
            .await?
            .ok_or_else(|| invalid_input_error())?
            .try_get("data")?;

        Ok(driver)
    }

    #[tracing::instrument(skip(self))]
    async fn start_driver(&self, user: User, id: Uuid) -> Result<Driver, Error> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;

        let mut driver = fetch_driver_for_update(&mut tx, &id).await?;

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

        driver.stop()?;

        update_driver(&mut tx, &driver).await?;

        tx.commit().await?;

        Ok(driver)
    }

    #[tracing::instrument(skip(self))]
    async fn update_driver_location(
        &self,
        user: User,
        id: Uuid,
        coordinates: Coordinates,
    ) -> Result<(), Error> {
        let mut conn = self.pool.acquire().await?;

        let coordinates: Geometry<f64> = coordinates.into();

        conn.execute(
            sqlx::query(
                "UPDATE driver_locations SET location = ST_SetSRID($2, 4326), expiry = $3 WHERE driver_id = $1",
            )
            .bind(&id)
            .bind(wkb::Encode(coordinates))
            .bind(Utc::now() + Duration::seconds(60)),
        )
        .await?;

        Ok(())
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
