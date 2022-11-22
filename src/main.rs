use dotenv::dotenv;

use caballus::db::PgPool;
use caballus::engine::Engine;
use caballus::server::serve;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let PgPool(pool) = PgPool::new("postgresql://caballus:caballus@localhost:5432/caballus", 5)
        .await
        .unwrap();

    let engine = Engine::new(pool).await.unwrap();

    serve(engine).await;
}
