mod handlers;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::Extension,
    routing::{get, patch, post},
    Router,
};

use crate::server::handlers::{drivers, google_places, locations, quotes, routes, trips};
use crate::{api::API, auth::User};

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
        .route("/trips/:id/driver/request", patch(trips::request_driver))
        .route("/trips/:id/driver/release", patch(trips::release_driver))
        .route("/trips/:id/driver/accept", patch(trips::accept_trip))
        .route("/trips/:id/driver/reject", patch(trips::reject_trip))
        .route("/trips/:id/cancel", patch(trips::cancel))
        .route("/drivers", post(drivers::create))
        .route("/drivers/:id", get(drivers::find))
        .route("/drivers/:id/start", patch(drivers::start))
        .route("/drivers/:id/stop", patch(drivers::stop))
        .route("/drivers/:id/location", patch(drivers::update_location))
        .route("/drivers/:id/rate", patch(drivers::update_rate))
        .route(
            "/google_places/suggestions",
            get(google_places::find_suggestions),
        )
        .route("/google_places/:id", get(google_places::find))
        .layer(Extension(api))
        .layer(Extension(User::new_system_user()));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    tracing::info!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
