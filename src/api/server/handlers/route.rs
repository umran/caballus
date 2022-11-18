use axum::extract::{Extension, Json, Path};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::interface::DynAPI,
    entities::{Place, Route},
    error::Error,
};

#[derive(Serialize, Deserialize)]
pub struct CreateRouteParams {
    origin: Place,
    destination: Place,
}

pub async fn create(
    Extension(api): Extension<DynAPI>,
    Json(params): Json<CreateRouteParams>,
) -> Result<Json<Route>, Error> {
    let route = api.create_route(params.origin, params.destination).await?;

    Ok(route.into())
}

pub async fn find(
    Extension(api): Extension<DynAPI>,
    Path(id): Path<Uuid>,
) -> Result<Json<Route>, Error> {
    let route = api.find_route(id).await?;

    Ok(route.into())
}
