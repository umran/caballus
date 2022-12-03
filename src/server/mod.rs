mod handlers;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::Extension,
    routing::{get, post, put},
    Router,
};

use crate::api::API;
use crate::server::handlers::{drivers, google_places, locations, quotes, routes, trips};

type DynAPI = Arc<dyn API + Send + Sync>;

pub async fn serve<T: API + Sync + Send + 'static>(api: T) {
    let api = Arc::new(api) as DynAPI;

    let app = Router::new()
        .route("/locations", post(locations::create))
        .route("/locations/:token", get(locations::find))
        .route("/routes", post(routes::create))
        .route("/routes/:token", get(routes::find))
        .route("/quotes", post(quotes::create))
        .route("/quotes/:token", get(quotes::find))
        .route("/trips", post(trips::create))
        .route("/trips/:id", get(trips::find))
        .route("/trips/:id/driver/request", put(trips::request_driver))
        .route("/trips/:id/driver/derequest", put(trips::derequest_driver))
        .route("/trips/:id/driver/assign", put(trips::assign_driver))
        .route("/trips/:id/cancel", put(trips::cancel))
        .route("/drivers", put(drivers::create))
        .route("/drivers/:id", get(drivers::find))
        .route("/drivers/:id/start", put(drivers::start))
        .route("/drivers/:id/stop", put(drivers::stop))
        .route("/drivers/:id/location", put(drivers::update_location))
        .route("/drivers/:id/rate", put(drivers::update_rate))
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
