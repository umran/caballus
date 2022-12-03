use axum::extract::{Extension, Json, Path};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entities::{Location, LocationSource};
use crate::error::Error;
use crate::server::DynAPI;

#[derive(Serialize, Deserialize)]
pub struct CreateParams {
    source: LocationSource,
}

pub async fn create(
    Extension(api): Extension<DynAPI>,
    Json(params): Json<CreateParams>,
) -> Result<Json<Location>, Error> {
    let location = api.create_location(params.source).await?;

    Ok(location.into())
}

pub async fn find(
    Extension(api): Extension<DynAPI>,
    Path(token): Path<Uuid>,
) -> Result<Json<Location>, Error> {
    let location = api.find_location(token).await?;

    Ok(location.into())
}
