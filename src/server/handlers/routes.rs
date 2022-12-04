use axum::extract::{Extension, Json, Path};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::User;
use crate::server::DynAPI;
use crate::{entities::Route, error::Error};

#[derive(Serialize, Deserialize)]
pub struct CreateParams {
    origin_id: Uuid,
    destination_id: Uuid,
}

pub async fn create(
    Extension(api): Extension<DynAPI>,
    Extension(user): Extension<User>,
    Json(params): Json<CreateParams>,
) -> Result<Json<Route>, Error> {
    let route = api
        .create_route(user, params.origin_id, params.destination_id)
        .await?;

    Ok(route.into())
}

pub async fn find(
    Extension(api): Extension<DynAPI>,
    Extension(user): Extension<User>,
    Path(token): Path<Uuid>,
) -> Result<Json<Route>, Error> {
    let route = api.find_route(user, token).await?;

    Ok(route.into())
}
