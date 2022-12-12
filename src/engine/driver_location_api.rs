use super::Engine;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use geo_types::Geometry;
use geozero::wkb;
use sqlx::Executor;
use uuid::Uuid;

use crate::{api::DriverLocationAPI, auth::User, entities::Coordinates, error::Error};

#[async_trait]
impl DriverLocationAPI for Engine {
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
}
