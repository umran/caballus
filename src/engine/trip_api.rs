use super::helpers::{
    fetch_driver_for_update, fetch_passenger_for_update, fetch_trip_for_update, update_driver,
    update_passenger, update_trip,
};
use super::{Database, Engine};

use async_trait::async_trait;
use sqlx::{types::Json, Acquire, Executor, Row, Transaction};
use uuid::Uuid;

use crate::api::DriverSearchAPI;
use crate::{
    api::{QuoteAPI, TripAPI},
    auth::{Platform, User},
    entities::Trip,
    error::{invalid_input_error, invalid_invocation_error, Error},
};

#[async_trait]
impl TripAPI for Engine {
    #[tracing::instrument(skip(self))]
    async fn create_trip(&self, user: User, quote_token: Uuid) -> Result<Trip, Error> {
        self.authorize(user.clone(), "create_trip", Platform::default())?;

        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;

        let quote = self.find_quote(user.clone(), quote_token).await?;
        let trip = Trip::new(user.id.clone(), quote.route, quote.max_fare);

        // ensure passenger does not have another active trip while trip is created
        let mut passenger = fetch_passenger_for_update(&mut tx, &trip.passenger_id).await?;
        passenger.activate(trip.id.clone())?;

        tx.execute(
            sqlx::query("INSERT INTO trips (id, status, data) VALUES ($1, $2, $3)")
                .bind(&trip.id)
                .bind(&trip.status.name())
                .bind(Json(&trip)),
        )
        .await?;

        update_passenger(&mut tx, &passenger).await?;

        tx.commit().await?;

        Ok(trip)
    }

    #[tracing::instrument(skip(self))]
    async fn find_trip(&self, user: User, id: Uuid) -> Result<Trip, Error> {
        let mut conn = self.pool.acquire().await?;

        let maybe_result = conn
            .fetch_optional(sqlx::query("SELECT data FROM trips WHERE id = $1").bind(&id))
            .await?;

        let result = maybe_result.ok_or_else(|| invalid_input_error())?;
        let Json(trip): Json<Trip> = result.try_get("data")?;

        self.authorize(user.clone(), "read", trip.clone())?;

        Ok(trip)
    }

    #[tracing::instrument(skip(self))]
    async fn request_driver(&self, user: User, id: Uuid) -> Result<Option<Trip>, Error> {
        let mut conn = self.pool.acquire().await?;

        // fetch trip
        tracing::info!("fetching trip without lock");

        let Json(trip): Json<Trip> = sqlx::query("SELECT data FROM trips WHERE id = $1")
            .bind(&id)
            .fetch_optional(&mut conn)
            .await?
            .ok_or_else(|| invalid_input_error())?
            .try_get("data")?;

        // it's safe to perform the authorization check without locking on trip
        self.authorize(user.clone(), "request_driver", trip.clone())?;

        if !trip.is_searching() {
            tracing::info!("trip is not in the SEARCHING state, returning early...");
            return Err(invalid_invocation_error());
        }

        // find drivers
        let drivers = self.find_drivers(user.clone(), trip.clone()).await?;

        tracing::info!(
            "iterating through drivers to find a driver that satisfies all conditions..."
        );

        for (driver_id, distance) in drivers.into_iter() {
            let mut tx = conn.begin().await?;

            let mut trip = fetch_trip_for_update(&mut tx, &id).await?;
            let mut driver = fetch_driver_for_update(&mut tx, &driver_id).await?;

            if !driver.is_available() {
                continue;
            }

            let (min_fare, rate): (f64, f64) = sqlx::query_as(
                "SELECT min_fare, rate FROM driver_rates WHERE driver_id = $1 FOR UPDATE",
            )
            .bind(&driver.id)
            .fetch_one(&mut tx)
            .await?;

            let fare = f64::max(min_fare, (distance + trip.route.distance) * rate);

            if fare > trip.max_fare {
                continue;
            }

            let maybe_trip_rejection = sqlx::query("SELECT driver_id FROM trip_rejections WHERE trip_id = $1 AND driver_id = $2 FOR UPDATE").bind(&trip.id).bind(&driver.id).fetch_optional(&mut tx).await?;
            if maybe_trip_rejection.is_some() {
                continue;
            }

            tracing::info!(
                "driver satisfies all conditions, attempting to update trip and driver..."
            );

            driver.request(trip.id.clone())?;
            trip.request_driver(driver_id.clone(), fare)?;

            update_driver(&mut tx, &driver).await?;
            update_trip(&mut tx, &trip).await?;

            tx.commit().await?;

            tracing::info!("successfully requested driver, returning...");

            return Ok(Some(trip));
        }

        tracing::warn!(
            "failed to request a driver as no drivers satisfied all conditions, returning..."
        );

        Ok(None)
    }

    async fn release_driver(&self, user: User, id: Uuid, driver_id: Uuid) -> Result<Trip, Error> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;

        let mut trip = fetch_trip_for_update(&mut tx, &id).await?;

        self.authorize(user.clone(), "release_driver", trip.clone())?;

        release_driver(&mut tx, &mut trip, driver_id, false).await?;

        tx.commit().await?;

        Ok(trip)
    }

    async fn accept_trip(&self, user: User, id: Uuid) -> Result<Trip, Error> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;

        let mut trip = fetch_trip_for_update(&mut tx, &id).await?;

        self.authorize(user.clone(), "accept_trip", trip.clone())?;

        trip.assign_driver()?;

        let mut driver = fetch_driver_for_update(&mut tx, &user.id).await?;

        driver.assign()?;

        update_trip(&mut tx, &trip).await?;
        update_driver(&mut tx, &driver).await?;

        sqlx::query("UPDATE driver_priorities SET priority = GREATEST(0, priority - 1) WHERE driver_id = $1")
            .bind(&driver.id)
            .execute(&mut tx)
            .await?;

        tx.commit().await?;

        Ok(trip)
    }

    async fn reject_trip(&self, user: User, id: Uuid) -> Result<Trip, Error> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;

        let mut trip = fetch_trip_for_update(&mut tx, &id).await?;

        self.authorize(user.clone(), "reject_trip", trip.clone())?;

        release_driver(&mut tx, &mut trip, user.id.clone(), true).await?;

        tx.commit().await?;

        Ok(trip)
    }

    async fn cancel_trip(&self, user: User, id: Uuid) -> Result<Trip, Error> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;

        let mut trip = fetch_trip_for_update(&mut tx, &id).await?;

        self.authorize(user.clone(), "cancel_trip", trip.clone())?;

        let is_passenger = user.id == trip.passenger_id;

        let freed_driver = trip.cancel(is_passenger)?;

        update_trip(&mut tx, &trip).await?;

        if let Some(driver_id) = freed_driver {
            let mut driver = fetch_driver_for_update(&mut tx, &driver_id).await?;
            driver.free()?;

            update_driver(&mut tx, &driver).await?;
        }

        let mut passenger = fetch_passenger_for_update(&mut tx, &trip.passenger_id).await?;
        passenger.deactivate()?;

        update_passenger(&mut tx, &passenger).await?;

        tx.commit().await?;

        Ok(trip)
    }

    async fn report_origin_arrival(&self, user: User, id: Uuid) -> Result<Trip, Error> {
        unimplemented!()
    }

    async fn report_destination_arrival(&self, user: User, id: Uuid) -> Result<Trip, Error> {
        unimplemented!()
    }
}

async fn release_driver(
    tx: &mut Transaction<'_, Database>,
    trip: &mut Trip,
    driver_id: Uuid,
    rejection: bool,
) -> Result<(), Error> {
    if trip.release_driver()? != driver_id {
        return Err(invalid_invocation_error());
    }

    let mut driver = fetch_driver_for_update(tx, &driver_id).await?;

    update_trip(tx, &trip).await?;
    driver.free()?;

    update_driver(tx, &driver).await?;

    if rejection {
        tx.execute(
            sqlx::query("INSERT INTO trip_rejections (trip_id, driver_id) VALUES ($1, $2)")
                .bind(&trip.id)
                .bind(&driver.id),
        )
        .await?;

        tx.execute(sqlx::query("UPDATE driver_priorities SET priority = GREATEST(0, priority - 1) WHERE driver_id = $1").bind(&driver.id)).await?;
    } else {
        tx.execute(sqlx::query("UPDATE driver_priorities SET priority = LEAST(1, priority + 1) WHERE driver_id = $1").bind(&driver.id)).await?;
    }

    Ok(())
}
