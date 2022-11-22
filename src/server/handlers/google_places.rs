use axum::extract::{Json, Path, Query};
use serde::{Deserialize, Serialize};

use crate::{
    error::Error,
    external::google_maps::{find_place, find_place_suggestions, Place, PlaceSuggestions},
};

#[derive(Serialize, Deserialize)]
pub struct FindSuggestionsParams {
    input: String,
    location: String,
    radius: f64,
    session_token: String,
}

pub async fn find_suggestions(
    Query(params): Query<FindSuggestionsParams>,
) -> Result<Json<PlaceSuggestions>, Error> {
    let data = find_place_suggestions(
        params.input,
        params.location.try_into()?,
        params.radius,
        params.session_token,
    )
    .await?;

    Ok(data.into())
}

#[derive(Serialize, Deserialize)]
pub struct FindParams {
    session_token: String,
}

pub async fn find(
    Path(id): Path<String>,
    Query(params): Query<FindParams>,
) -> Result<Json<Place>, Error> {
    let data = find_place(id, params.session_token).await?;

    Ok(data.into())
}
