use axum::extract::{Extension, Json, Path};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::User;
use crate::entities::Trip;
use crate::error::Error;
use crate::server::DynAPI;

#[derive(Serialize, Deserialize)]
pub struct CreateParams {
    quote_token: Uuid,
}

#[derive(Serialize, Deserialize)]
pub struct ReleaseDriverParams {
    driver_id: Uuid,
}

pub async fn create(
    Extension(api): Extension<DynAPI>,
    Extension(user): Extension<User>,
    Json(params): Json<CreateParams>,
) -> Result<Json<Trip>, Error> {
    let trip = api.create_trip(user, params.quote_token).await?;

    Ok(trip.into())
}

pub async fn find(
    Extension(api): Extension<DynAPI>,
    Extension(user): Extension<User>,
    Path(id): Path<Uuid>,
) -> Result<Json<Trip>, Error> {
    let trip = api.find_trip(user, id).await?;

    Ok(trip.into())
}

pub async fn request_driver(
    Extension(api): Extension<DynAPI>,
    Extension(user): Extension<User>,
    Path(id): Path<Uuid>,
) -> Result<Json<Option<Trip>>, Error> {
    let trip = api.request_driver(user, id).await?;

    Ok(trip.into())
}

pub async fn release_driver(
    Extension(api): Extension<DynAPI>,
    Extension(user): Extension<User>,
    Path(id): Path<Uuid>,
    Json(params): Json<ReleaseDriverParams>,
) -> Result<Json<Trip>, Error> {
    let trip = api.release_driver(user, id, params.driver_id).await?;

    Ok(trip.into())
}

pub async fn accept_trip(
    Extension(api): Extension<DynAPI>,
    Extension(user): Extension<User>,
    Path(id): Path<Uuid>,
) -> Result<Json<Trip>, Error> {
    let trip = api.accept_trip(user, id).await?;

    Ok(trip.into())
}

pub async fn reject_trip(
    Extension(api): Extension<DynAPI>,
    Extension(user): Extension<User>,
    Path(id): Path<Uuid>,
) -> Result<Json<Trip>, Error> {
    let trip = api.reject_trip(user, id).await?;

    Ok(trip.into())
}

pub async fn cancel(
    Extension(api): Extension<DynAPI>,
    Extension(user): Extension<User>,
    Path(id): Path<Uuid>,
) -> Result<Json<Trip>, Error> {
    let trip = api.cancel_trip(user, id).await?;

    Ok(trip.into())
}
