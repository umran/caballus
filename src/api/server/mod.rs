mod handlers;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::Extension,
    routing::{get, post},
    Router,
};

use crate::api::server::handlers::{google_places, routes};
use crate::api::{interface::DynAPI, API};

pub async fn serve<T: API + Sync + Send + 'static>(api: T) {
    tracing_subscriber::fmt::init();

    let api = Arc::new(api) as DynAPI;

    let app = Router::new()
        .route("/routes", post(routes::create))
        .route("/routes/:id", get(routes::find))
        .route(
            "/google_places/suggestions",
            get(google_places::find_suggestions),
        )
        .route("/google_places/:id", get(google_places::find))
        .layer(Extension(api));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    tracing::info!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
