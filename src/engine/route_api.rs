use super::Engine;

use async_trait::async_trait;
use serde_json::json;
use sqlx::{types::Json, Executor, Row};
use uuid::Uuid;

use crate::{
    api::{LocationAPI, RouteAPI},
    auth::User,
    entities::Route,
    error::Error,
};

#[async_trait]
impl RouteAPI for Engine {
    #[tracing::instrument(skip(self))]
    async fn create_route(
        &self,
        user: User,
        origin_token: Uuid,
        destination_token: Uuid,
    ) -> Result<Route, Error> {
        let origin = self.find_location(user.clone(), origin_token).await?;
        let destination = self.find_location(user.clone(), destination_token).await?;

        let route = Route::new(origin, destination, json!(""), 4500.0);

        let mut conn = self.pool.acquire().await?;
        conn.execute(
            sqlx::query("INSERT INTO routes (token, data) VALUES ($1, $2)")
                .bind(&route.token)
                .bind(Json(&route)),
        )
        .await?;

        Ok(route)
    }

    #[tracing::instrument(skip(self))]
    async fn find_route(&self, user: User, token: Uuid) -> Result<Route, Error> {
        let mut conn = self.pool.acquire().await?;

        let maybe_result = conn
            .fetch_optional(sqlx::query("SELECT data FROM routes WHERE token = $1").bind(&token))
            .await?;

        let result = maybe_result.ok_or_else(|| Error::invalid_input_error())?;
        let Json(route) = result.try_get("data")?;

        Ok(route)
    }
}
