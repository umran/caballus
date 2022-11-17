use sqlx::{postgres::PgPoolOptions, Error, Pool, Postgres};

pub struct PgPool(pub Pool<Postgres>);

impl PgPool {
    pub async fn new(db_uri: &str, max_connections: u32) -> Result<Self, Error> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .connect(db_uri)
            .await?;

        Ok(Self(pool))
    }
}
