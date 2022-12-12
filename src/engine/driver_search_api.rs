use super::Engine;

use async_trait::async_trait;
use geo_types::Geometry;
use geozero::wkb;
use sqlx::{Executor, Row};
use uuid::Uuid;

use crate::{
    api::DriverSearchAPI,
    auth::User,
    entities::{Driver, Trip},
    error::Error,
};

#[async_trait]
impl DriverSearchAPI for Engine {
    async fn synchronize_drivers(&self, user: User, drivers: Vec<Driver>) -> Result<(), Error> {
        unimplemented!()
    }

    async fn find_drivers(&self, user: User, trip: Trip) -> Result<Vec<(Uuid, f64)>, Error> {
        let origin_location: Geometry<f64> = trip.route.origin.coordinates.clone().into();
        let trip_distance = trip.route.distance;
        let search_radius = 2000.0;

        // fetch potential driver ids for trip
        let query = "
            SELECT
                d.id AS driver_id,
                ST_Distance(l.location, ST_SetSRID($1, 4326)) as distance
            FROM
                drivers d
                LEFT JOIN driver_rates r ON d.id = r.driver_id
                LEFT JOIN driver_locations l ON d.id = l.driver_id
                LEFT JOIN driver_priorities p ON d.id = p.driver_id
                LEFT JOIN trip_rejections tr ON tr.trip_id = $5 AND d.id = tr.driver_id
            WHERE
                d.status = 'available'
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

        let mut conn = self.pool.acquire().await?;
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

        let mut fares = vec![];

        for result in results.iter() {
            let driver_id: Uuid = result.try_get("driver_id")?;
            let fare: f64 = result.try_get("fare")?;

            fares.push((driver_id, fare));
        }

        Ok(fares)
    }
}
