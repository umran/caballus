mod driver_api;
mod driver_location_api;
mod driver_search_api;
mod helpers;
mod location_api;
mod quote_api;
mod route_api;
mod trip_api;

use oso::Oso;
use sqlx::{Executor, Pool, Postgres};

use crate::{
    api::API,
    auth::authorizor,
    error::{unauthorized_error, Error},
};

type Database = Postgres;

pub struct Engine {
    pool: Pool<Database>,
    authorizor: Oso,
}

impl Engine {
    #[tracing::instrument(name = "Engine::new", skip_all)]
    pub async fn new(pool: Pool<Database>) -> Result<Self, Error> {
        // location service (KV store)
        pool.execute("DROP TABLE IF EXISTS locations CASCADE")
            .await?;
        pool.execute("CREATE TABLE locations (token UUID PRIMARY KEY, data JSONB NOT NULL)")
            .await?;

        // route service (KV store)
        pool.execute("DROP TABLE IF EXISTS routes CASCADE").await?;
        pool.execute("CREATE TABLE routes (token UUID PRIMARY KEY, data JSONB NOT NULL)")
            .await?;

        // quote service (KV store)
        pool.execute("DROP TABLE IF EXISTS quotes CASCADE").await?;
        pool.execute("CREATE TABLE quotes (token UUID PRIMARY KEY, data JSONB NOT NULL)")
            .await?;

        // trip service
        pool.execute("DROP TABLE IF EXISTS trips CASCADE").await?;
        pool.execute("CREATE TABLE trips (id UUID PRIMARY KEY, status VARCHAR NOT NULL, data JSONB NOT NULL)")
            .await?;

        pool.execute("DROP TABLE IF EXISTS trip_rejections CASCADE")
            .await?;
        pool.execute("CREATE TABLE trip_rejections (trip_id UUID NOT NULL, driver_id UUID NOT NULL, PRIMARY KEY (trip_id, driver_id))")
            .await?;

        pool.execute("DROP TABLE IF EXISTS passengers CASCADE")
            .await?;
        pool.execute("CREATE TABLE passengers (id UUID PRIMARY KEY, status VARCHAR NOT NULL, data JSONB NOT NULL)")
                .await?;

        pool.execute("DROP TABLE IF EXISTS drivers CASCADE").await?;
        pool.execute("CREATE TABLE drivers (id UUID PRIMARY KEY, status VARCHAR NOT NULL, data JSONB NOT NULL)")
            .await?;

        pool.execute("DROP TABLE IF EXISTS driver_rates CASCADE")
            .await?;
        pool.execute(
            "CREATE TABLE driver_rates (driver_id UUID PRIMARY KEY, min_fare DECIMAL, rate DECIMAL)",
        )
        .await?;

        pool.execute("DROP TABLE IF EXISTS driver_locations CASCADE")
            .await?;
        pool.execute("CREATE TABLE driver_locations (driver_id UUID PRIMARY KEY, location geometry(Point), expiry TIMESTAMP)")
            .await?;

        pool.execute("DROP TABLE IF EXISTS driver_priorities CASCADE")
            .await?;
        pool.execute(
            "CREATE TABLE driver_priorities (driver_id UUID PRIMARY KEY, priority INT4 NOT NULL)",
        )
        .await?;

        Ok(Self {
            pool,
            authorizor: authorizor::new(),
        })
    }
}

impl Engine {
    pub fn authorize<Actor, Action, Resource>(
        &self,
        actor: Actor,
        action: Action,
        resource: Resource,
    ) -> Result<(), Error>
    where
        Actor: oso::ToPolar,
        Action: oso::ToPolar,
        Resource: oso::ToPolar,
    {
        if self.authorizor.is_allowed(actor, action, resource)? {
            return Ok(());
        }

        Err(unauthorized_error())
    }
}

impl API for Engine {}
