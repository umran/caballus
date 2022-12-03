use axum::extract::{Extension, Json, Path};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entities::Trip;
use crate::error::Error;
use crate::server::DynAPI;

#[derive(Serialize, Deserialize)]
pub struct CreateParams {
    quote_token: Uuid,
    user_id: Uuid,
}

#[derive(Serialize, Deserialize)]
pub struct DerequestDriverParams {
    user_id: Uuid,
    rejected: bool,
}

#[derive(Serialize, Deserialize)]
pub struct AssignDriverParams {
    user_id: Uuid,
}

#[derive(Serialize, Deserialize)]
pub struct CancelParams {
    user_id: Option<Uuid>,
}

pub async fn create(
    Extension(api): Extension<DynAPI>,
    Json(params): Json<CreateParams>,
) -> Result<Json<Trip>, Error> {
    let trip = api.create_trip(params.quote_token, params.user_id).await?;

    Ok(trip.into())
}

pub async fn find(
    Extension(api): Extension<DynAPI>,
    Path(id): Path<Uuid>,
) -> Result<Json<Trip>, Error> {
    let trip = api.find_trip(id).await?;

    Ok(trip.into())
}

pub async fn request_driver(
    Extension(api): Extension<DynAPI>,
    Path(id): Path<Uuid>,
) -> Result<Json<Option<Trip>>, Error> {
    let trip = api.request_driver(id).await?;

    Ok(trip.into())
}

pub async fn derequest_driver(
    Extension(api): Extension<DynAPI>,
    Path(id): Path<Uuid>,
    Json(params): Json<DerequestDriverParams>,
) -> Result<Json<Trip>, Error> {
    let trip = api
        .derequest_driver(id, params.user_id, params.rejected)
        .await?;

    Ok(trip.into())
}

pub async fn assign_driver(
    Extension(api): Extension<DynAPI>,
    Path(id): Path<Uuid>,
    Json(params): Json<AssignDriverParams>,
) -> Result<Json<Trip>, Error> {
    let trip = api.assign_driver(id, params.user_id).await?;

    Ok(trip.into())
}

pub async fn cancel(
    Extension(api): Extension<DynAPI>,
    Path(id): Path<Uuid>,
    Json(params): Json<CancelParams>,
) -> Result<Json<Trip>, Error> {
    let trip = api.cancel_trip(id, params.user_id).await?;

    Ok(trip.into())
}
