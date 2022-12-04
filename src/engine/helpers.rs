use super::Database;

use sqlx::{types::Json, Executor, Row, Transaction};
use uuid::Uuid;

use crate::{
    entities::{Driver, Trip},
    error::{invalid_input_error, Error},
};

#[tracing::instrument(skip(tx))]
pub async fn fetch_trip_for_update(
    tx: &mut Transaction<'_, Database>,
    id: &Uuid,
) -> Result<Trip, Error> {
    let Json(trip): Json<Trip> = tx
        .fetch_optional(sqlx::query("SELECT data FROM trips WHERE id = $1 FOR UPDATE").bind(id))
        .await?
        .ok_or_else(|| invalid_input_error())?
        .try_get("data")?;

    Ok(trip)
}

#[tracing::instrument(skip(tx))]
pub async fn fetch_driver_for_update(
    tx: &mut Transaction<'_, Database>,
    id: &Uuid,
) -> Result<Driver, Error> {
    let Json(driver): Json<Driver> = tx
        .fetch_optional(sqlx::query("SELECT data FROM drivers WHERE id = $1 FOR UPDATE").bind(id))
        .await?
        .ok_or_else(|| invalid_input_error())?
        .try_get("data")?;

    Ok(driver)
}

#[tracing::instrument(skip(tx))]
pub async fn update_trip(tx: &mut Transaction<'_, Database>, trip: &Trip) -> Result<(), Error> {
    tx.execute(
        sqlx::query("UPDATE trips SET status = $2, data = $3 WHERE id = $1")
            .bind(&trip.id)
            .bind(trip.status.name())
            .bind(Json(trip)),
    )
    .await?;

    Ok(())
}

#[tracing::instrument(skip(tx))]
pub async fn update_driver(
    tx: &mut Transaction<'_, Database>,
    driver: &Driver,
) -> Result<(), Error> {
    tx.execute(
        sqlx::query("UPDATE drivers SET status = $2, data = $3 WHERE id = $1")
            .bind(&driver.id)
            .bind(driver.status.name())
            .bind(Json(driver)),
    )
    .await?;

    Ok(())
}
