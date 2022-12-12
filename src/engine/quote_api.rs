use super::Engine;

use async_trait::async_trait;
use geo_types::Geometry;
use geozero::wkb;
use sqlx::{types::Json, Executor, Row};
use uuid::Uuid;

use crate::{
    api::{QuoteAPI, RouteAPI},
    auth::User,
    entities::Quote,
    error::{invalid_input_error, Error},
};

#[async_trait]
impl QuoteAPI for Engine {
    #[tracing::instrument(skip(self))]
    async fn create_quote(&self, user: User, route_token: Uuid) -> Result<Option<Quote>, Error> {
        let route = self.find_route(user.clone(), route_token).await?;

        let origin_location: Geometry<f64> = route.origin.coordinates.clone().into();
        let search_radius = 2000.0;

        let query = "
            SELECT
                percentile_cont(0.5) WITHIN GROUP (
                    ORDER BY
                        fares.fare ASC
                ) AS max_fare
            FROM
                (
                    SELECT
                        GREATEST(
                            r.min_fare, r.rate * (
                                ST_Distance(l.location, ST_SetSRID($1, 4326)) + $2
                            )
                        ) AS fare
                    FROM
                        drivers d
                        LEFT JOIN driver_rates r ON d.id = r.driver_id
                        LEFT JOIN driver_locations l ON d.id = l.driver_id
                    WHERE
                        d.status = 'available'
                        AND r.rate IS NOT NULL
                        AND l.location IS NOT NULL
                        AND l.expiry > now()
                        AND ST_DWithin(l.location, ST_SetSRID($1, 4326), $3)
                ) AS fares
        ";

        let mut conn = self.pool.acquire().await?;

        let maybe_max_fare: Option<f64> = conn
            .fetch_one(
                sqlx::query(query)
                    .bind(wkb::Encode(origin_location))
                    .bind(route.distance)
                    .bind(search_radius),
            )
            .await?
            .try_get("max_fare")?;

        match maybe_max_fare {
            Some(max_fare) => {
                let quote = Quote::new(route, max_fare);

                conn.execute(
                    sqlx::query("INSERT INTO quotes (token, data) VALUES ($1, $2)")
                        .bind(&quote.token)
                        .bind(Json(&quote)),
                )
                .await?;

                Ok(Some(quote))
            }
            None => Ok(None),
        }
    }

    #[tracing::instrument(skip(self))]
    async fn find_quote(&self, user: User, quote_token: Uuid) -> Result<Quote, Error> {
        let mut conn = self.pool.acquire().await?;

        let maybe_result = conn
            .fetch_optional(
                sqlx::query("SELECT data FROM quotes WHERE token = $1").bind(&quote_token),
            )
            .await?;

        let result = maybe_result.ok_or_else(|| invalid_input_error())?;
        let Json(quote) = result.try_get("data")?;

        Ok(quote)
    }
}
