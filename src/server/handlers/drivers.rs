use axum::extract::{Extension, Json, Path};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::User;
use crate::entities::{Coordinates, Driver};
use crate::error::Error;
use crate::server::DynAPI;

#[derive(Serialize, Deserialize)]
pub struct UpdateLocationParams {
    coordinates: Coordinates,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateRateParams {
    min_fare: f64,
    rate: f64,
}

pub async fn create(
    Extension(api): Extension<DynAPI>,
    Extension(user): Extension<User>,
) -> Result<Json<Driver>, Error> {
    let driver = api.create_driver(user).await?;

    Ok(driver.into())
}

pub async fn find(
    Extension(api): Extension<DynAPI>,
    Extension(user): Extension<User>,
    Path(id): Path<Uuid>,
) -> Result<Json<Driver>, Error> {
    let driver = api.find_driver(user, id).await?;

    Ok(driver.into())
}

pub async fn start(
    Extension(api): Extension<DynAPI>,
    Extension(user): Extension<User>,
    Path(id): Path<Uuid>,
) -> Result<Json<Driver>, Error> {
    let driver = api.start_driver(user, id).await?;

    Ok(driver.into())
}

pub async fn stop(
    Extension(api): Extension<DynAPI>,
    Extension(user): Extension<User>,
    Path(id): Path<Uuid>,
) -> Result<Json<Driver>, Error> {
    let driver = api.stop_driver(user, id).await?;

    Ok(driver.into())
}

pub async fn update_location(
    Extension(api): Extension<DynAPI>,
    Extension(user): Extension<User>,
    Path(id): Path<Uuid>,
    Json(params): Json<UpdateLocationParams>,
) -> Result<Json<()>, Error> {
    api.update_driver_location(user, id, params.coordinates)
        .await?;

    Ok(().into())
}

pub async fn update_rate(
    Extension(api): Extension<DynAPI>,
    Extension(user): Extension<User>,
    Path(id): Path<Uuid>,
    Json(params): Json<UpdateRateParams>,
) -> Result<Json<()>, Error> {
    api.update_driver_rate(user, id, params.min_fare, params.rate)
        .await?;

    Ok(().into())
}
