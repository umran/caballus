use async_trait::async_trait;
use sqlx::{types::Json, Acquire, Executor};

use super::Engine;

use crate::{api::PassengerAPI, auth::User, entities::Passenger, error::Error};

#[async_trait]
impl PassengerAPI for Engine {
    async fn create_passenger(&self, user: User) -> Result<Passenger, Error> {
        let passenger = Passenger::new(user.id);

        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;

        tx.execute(
            sqlx::query("INSERT INTO passengers (id, status, data) VALUES ($1, $2, $3)")
                .bind(&passenger.id)
                .bind(&passenger.status.name())
                .bind(Json(&passenger)),
        )
        .await?;

        tx.commit().await?;

        Ok(passenger)
    }
}
