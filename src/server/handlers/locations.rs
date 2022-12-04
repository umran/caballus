use axum::extract::{Extension, Json, Path};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::User;
use crate::entities::{Location, LocationSource};
use crate::error::Error;
use crate::server::DynAPI;

#[derive(Serialize, Deserialize)]
pub struct CreateParams {
    source: LocationSource,
}

pub async fn create(
    Extension(api): Extension<DynAPI>,
    Extension(user): Extension<User>,
    Json(params): Json<CreateParams>,
) -> Result<Json<Location>, Error> {
    let location = api.create_location(user, params.source).await?;

    Ok(location.into())
}

pub async fn find(
    Extension(api): Extension<DynAPI>,
    Extension(user): Extension<User>,
    Path(token): Path<Uuid>,
) -> Result<Json<Location>, Error> {
    let location = api.find_location(user, token).await?;

    Ok(location.into())
}
