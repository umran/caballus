use axum::extract::{Json, Query};

use crate::{
    entities::Coordinates,
    error::Error,
    external::google_maps::{list_place_suggestions, PlaceSuggestions},
};

pub async fn list_suggestions(
    Query(input): Query<String>,
    Query(location): Query<Coordinates>,
    Query(radius): Query<f64>,
    Query(session_token): Query<String>,
) -> Result<Json<PlaceSuggestions>, Error> {
    let data = list_place_suggestions(input, location, radius, session_token).await?;

    Ok(data.into())
}
