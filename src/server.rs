use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::Extension,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::api::API;
use crate::db::PgStore;
use crate::engine::Engine;

pub async fn start() {
    tracing_subscriber::fmt::init();

    // set up API engine
    let pg_store = PgStore::new("postgresql://caballus:caballus@localhost:5432/caballus", 5)
        .await
        .unwrap();

    let api_engine = Arc::new(Engine::new(pg_store)) as State;

    let app = Router::new()
        .route("/", get(root))
        .layer(Extension(api_engine));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

type State = Arc<dyn API + Send + Sync>;

async fn root(Extension(state): Extension<State>) -> &'static str {
    "Hello, World!"
}
