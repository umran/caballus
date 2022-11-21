use serde::{Deserialize, Serialize};
use std::env;

use crate::{
    entities::Coordinates,
    error::{invalid_input_error, upstream_error, Error},
};

#[derive(Serialize, Deserialize)]
pub struct PlaceSuggestion {
    place_id: String,
    description: String,
}

pub type PlaceSuggestions = Vec<PlaceSuggestion>;

pub async fn list_place_suggestions(
    input: String,
    location: Coordinates,
    radius: f64,
    session_token: String,
) -> Result<Vec<PlaceSuggestion>, Error> {
    let location: String = location.into();

    let api_base = env::var("GOOGLE_MAPS_API_BASE")?;
    let url = format!("https://{}/maps/api/place/autocomplete/json", api_base);

    let key = env::var("GOOGLE_MAPS_API_KEY")?;

    let res = reqwest::Client::new()
        .get(url)
        .query(&[("key", &key)])
        .query(&["input", &input])
        .query(&["location", &location])
        .query(&[("radius", radius)])
        .query(&["sessiontoken", &session_token])
        .send()
        .await?;

    if res.status() == reqwest::StatusCode::BAD_GATEWAY {
        return Err(upstream_error());
    }

    let status_code = res.status().as_u16();
    if status_code >= 500 {
        return Err(upstream_error());
    } else if status_code != 200 {
        return Err(invalid_input_error());
    }

    let data: PlaceSuggestions = res.json().await?;
    Ok(data)
}
