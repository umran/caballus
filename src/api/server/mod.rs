mod handlers;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::Extension,
    routing::{get, post},
    Router,
};

use crate::api::server::handlers::route;
use crate::api::{interface::DynAPI, API};

pub async fn serve<T: API + Sync + Send + 'static>(api: T) {
    tracing_subscriber::fmt::init();

    let api = Arc::new(api) as DynAPI;

    let app = Router::new()
        .route("/routes", post(route::create))
        .route("/routes/:id", get(route::find))
        .layer(Extension(api));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    tracing::info!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
