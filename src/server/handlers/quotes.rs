use axum::extract::{Extension, Json, Path};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entities::Quote;
use crate::error::Error;
use crate::server::DynAPI;

#[derive(Serialize, Deserialize)]
pub struct CreateParams {
    route_token: Uuid,
}

pub async fn create(
    Extension(api): Extension<DynAPI>,
    Json(params): Json<CreateParams>,
) -> Result<Json<Option<Quote>>, Error> {
    let quote = api.create_quote(params.route_token).await?;

    Ok(quote.into())
}

pub async fn find(
    Extension(api): Extension<DynAPI>,
    Path(token): Path<Uuid>,
) -> Result<Json<Quote>, Error> {
    let quote = api.find_quote(token).await?;

    Ok(quote.into())
}
