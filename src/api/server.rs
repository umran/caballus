use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::{Extension, Path},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::API,
    entities::{Place, Route},
    error::Error,
};

use super::interface::DynAPI;

pub async fn serve<T: API + Sync + Send + 'static>(api: T) {
    tracing_subscriber::fmt::init();

    let api = Arc::new(api) as DynAPI;

    let app = Router::new()
        .route("/", get(root))
        .route("/routes/:id", get(find_route))
        .layer(Extension(api));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    tracing::info!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Serialize, Deserialize)]
struct CreateRouteParams {
    origin: Place,
    destination: Place,
}

async fn root() -> &'static str {
    "Hello, World!"
}

async fn create_route(
    Extension(api): Extension<DynAPI>,
    Json(params): Json<CreateRouteParams>,
) -> Result<Json<Route>, Error> {
    let route = api.create_route(params.origin, params.destination).await?;

    Ok(route.into())
}

async fn find_route(
    Extension(api): Extension<DynAPI>,
    Path(id): Path<Uuid>,
) -> Result<Json<Route>, Error> {
    let route = api.find_route(id).await?;

    Ok(route.into())
}
