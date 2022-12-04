use axum::extract::{Extension, Json, Path};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::User;
use crate::entities::Quote;
use crate::error::Error;
use crate::server::DynAPI;

#[derive(Serialize, Deserialize)]
pub struct CreateParams {
    route_token: Uuid,
}

pub async fn create(
    Extension(api): Extension<DynAPI>,
    Extension(user): Extension<User>,
    Json(params): Json<CreateParams>,
) -> Result<Json<Option<Quote>>, Error> {
    let quote = api.create_quote(user, params.route_token).await?;

    Ok(quote.into())
}

pub async fn find(
    Extension(api): Extension<DynAPI>,
    Extension(user): Extension<User>,
    Path(token): Path<Uuid>,
) -> Result<Json<Quote>, Error> {
    let quote = api.find_quote(user, token).await?;

    Ok(quote.into())
}
