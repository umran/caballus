use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::{Extension, Path},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::engine::Engine;
use crate::{api::API, error::Error, route::Route};
use crate::{db::PgPool, route::Place};

pub async fn start() {
    tracing_subscriber::fmt::init();

    // set up API engine
    let PgPool(pool) = PgPool::new("postgresql://caballus:caballus@localhost:5432/caballus", 5)
        .await
        .unwrap();

    let api_engine = Arc::new(Engine::new(pool).await.unwrap()) as State;

    let app = Router::new()
        .route("/", get(root))
        .route("/routes/:id", get(find_route))
        .layer(Extension(api_engine));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

type State = Arc<dyn API + Send + Sync>;

#[derive(Serialize, Deserialize)]
struct CreateRouteParams {
    origin: Place,
    destination: Place,
}

async fn root(Extension(state): Extension<State>) -> &'static str {
    "Hello, World!"
}

async fn create_route(
    Extension(state): Extension<State>,
    Json(params): Json<CreateRouteParams>,
) -> Result<Json<Route>, Error> {
    let route = state
        .create_route(params.origin, params.destination)
        .await?;

    Ok(route.into())
}

async fn find_route(
    Extension(state): Extension<State>,
    Path(id): Path<String>,
) -> Result<Json<Route>, Error> {
    let route = state.find_route(id).await?;

    Ok(route.into())
}
