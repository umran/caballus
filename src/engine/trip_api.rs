use super::helpers::{fetch_driver_for_update, fetch_trip_for_update, update_driver, update_trip};
use super::{Database, Engine};

use async_trait::async_trait;
use geo_types::Geometry;
use geozero::wkb;
use sqlx::{types::Json, Acquire, Executor, Row, Transaction};
use uuid::Uuid;

use crate::{
    api::{QuoteAPI, TripAPI},
    auth::{Platform, User},
    entities::{Driver, Trip},
    error::{invalid_input_error, invalid_invocation_error, Error},
};

#[async_trait]
impl TripAPI for Engine {
    #[tracing::instrument(skip(self))]
    async fn create_trip(&self, user: User, quote_token: Uuid) -> Result<Trip, Error> {
        self.authorize(user.clone(), "create_trip", Platform::default())?;

        let passenger_id = user.id;
        let quote = self.find_quote(user.clone(), quote_token).await?;
        let trip = Trip::new(passenger_id, quote.route, quote.max_fare);

        let mut conn = self.pool.acquire().await?;

        // TODO: ensure passenger exists and does not have another active trip while trip is created

        conn.execute(
            sqlx::query("INSERT INTO trips (id, status, data) VALUES ($1, $2, $3)")
                .bind(&trip.id)
                .bind(&trip.status.name())
                .bind(Json(&trip)),
        )
        .await?;

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
        let Json(trip): Json<Trip> = conn
            .fetch_optional(sqlx::query("SELECT data FROM trips WHERE id = $1").bind(&id))
            .await?
            .ok_or_else(|| invalid_input_error())?
            .try_get("data")?;

        // it's safe to perform the authorization check without locking on trip
        self.authorize(user.clone(), "request_driver", trip.clone())?;

        if !trip.is_searching() {
            tracing::info!("trip is not in the SEARCHING state, returning early...");
            return Err(invalid_invocation_error());
        }

        let origin_location: Geometry<f64> = trip.route.origin.coordinates.clone().into();
        let trip_distance = trip.route.distance;
        let search_radius = 2000.0;

        // fetch potential driver ids for trip
        let query = "
            SELECT
                d.id AS driver_id
            FROM
                drivers d
                LEFT JOIN driver_rates r ON d.id = r.driver_id
                LEFT JOIN driver_locations l ON d.id = l.driver_id
                LEFT JOIN driver_priorities p ON d.id = p.driver_id
                LEFT JOIN trip_rejections tr ON tr.trip_id = $5 AND d.id = tr.driver_id
            WHERE
                d.status = 'AVAILABLE'
                AND tr.driver_id IS NULL
                AND r.rate IS NOT NULL
                AND l.location IS NOT NULL
                AND l.expiry > now()
                AND ST_DWithin(l.location, ST_SetSRID($1, 4326), $3)
                AND
                    GREATEST(
                        r.min_fare, r.rate * (
                            ST_Distance(l.location, ST_SetSRID($1, 4326)) + $2
                        )
                    ) <= $4
            ORDER BY
                p.priority ASC,
                ST_Distance(l.location, ST_SetSRID($1, 4326)) ASC
        ";

        tracing::info!("fetching potential drivers...");

        let results = conn
            .fetch_all(
                sqlx::query(query)
                    .bind(wkb::Encode(origin_location.clone()))
                    .bind(trip_distance)
                    .bind(search_radius)
                    .bind(trip.max_fare)
                    .bind(&trip.id),
            )
            .await?;

        tracing::info!(
            "iterating through drivers to find a driver that satisfies all conditions..."
        );

        for result in results.iter() {
            let driver_id: Uuid = result.try_get("driver_id")?;

            let mut tx = conn.begin().await?;
            let mut trip = fetch_trip_for_update(&mut tx, &id).await?;

            // fetch driver for update
            let query = "
                SELECT
                    d.data AS driver,
                    GREATEST(
                        r.min_fare, r.rate * (
                            ST_Distance(l.location, ST_SetSRID($2, 4326)) + $3
                        )
                    ) AS fare
                FROM
                    drivers d
                    LEFT JOIN driver_rates r ON d.id = r.driver_id
                    LEFT JOIN driver_locations l ON d.id = l.driver_id
                    LEFT JOIN trip_rejections tr ON tr.trip_id = $6 AND d.id = tr.driver_id
                WHERE
                    d.id = $1
                    AND d.status = 'AVAILABLE'
                    AND tr.driver_id IS NULL
                    AND r.rate IS NOT NULL
                    AND l.location IS NOT NULL
                    AND l.expiry > now()
                    AND ST_DWithin(l.location, ST_SetSRID($2, 4326), $4)
                    AND
                        GREATEST(
                            r.min_fare, r.rate * (
                                ST_Distance(l.location, ST_SetSRID($2, 4326)) + $3
                            )
                        ) <= $5
                FOR UPDATE OF d
            ";

            tracing::info!("fetching driver for update");

            let maybe_result = tx
                .fetch_optional(
                    sqlx::query(query)
                        .bind(&driver_id)
                        .bind(wkb::Encode(origin_location.clone()))
                        .bind(trip_distance)
                        .bind(search_radius)
                        .bind(trip.max_fare)
                        .bind(&trip.id),
                )
                .await?;

            if maybe_result.is_none() {
                tracing::info!(
                    "driver did not satisfy all conditions, moving on to next driver..."
                );
                continue;
            }

            tracing::info!(
                "driver satisfies all conditions, attempting to update trip and driver..."
            );

            // note that unwrap will never panic because it is never called if maybe_result is none
            let result = maybe_result.unwrap();

            let Json(mut driver): Json<Driver> = result.try_get("driver")?;
            let fare: f64 = result.try_get("fare")?;

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

        tx.execute(sqlx::query("UPDATE driver_priorities SET priority = GREATEST(0, priority - 1) WHERE driver_id = $1").bind(&driver.id)).await?;

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

        tx.commit().await?;

        Ok(trip)
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
