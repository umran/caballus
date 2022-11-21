use serde::{Deserialize, Serialize};
use std::env;

use crate::{
    entities::Coordinates,
    error::{invalid_input_error, unexpected_error, upstream_error, Error},
};

#[derive(Serialize, Deserialize)]
pub struct Place {
    place_id: String,
    formatted_address: String,
    geometry: Geometry,
}

#[derive(Serialize, Deserialize)]
pub struct Geometry {
    location: Coordinates,
}

#[derive(Serialize, Deserialize)]
pub struct PlaceSuggestion {
    place_id: String,
    description: String,
}

pub type PlaceSuggestions = Vec<PlaceSuggestion>;

#[derive(Serialize, Deserialize)]
struct Response<T> {
    status: String,
    result: Option<T>,
    results: Option<T>,
    predictions: Option<T>,
}

pub async fn find_place_suggestions(
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
        .query(&[("key", key)])
        .query(&[("input", input)])
        .query(&[("location", location)])
        .query(&[("radius", radius)])
        .query(&[("sessiontoken", session_token)])
        .send()
        .await?;

    let status_code = res.status().as_u16();

    if status_code >= 500 {
        return Err(upstream_error());
    } else if status_code != 200 {
        return Err(invalid_input_error());
    }

    let data: Response<PlaceSuggestions> = res.json().await?;

    if !(data.status == "OK" || data.status == "ZERO_RESULTS") {
        return Err(invalid_input_error());
    }

    Ok(data.predictions.ok_or_else(|| unexpected_error())?)
}

pub async fn find_place(id: String, session_token: String) -> Result<Place, Error> {
    let api_base = env::var("GOOGLE_MAPS_API_BASE")?;
    let url = format!("https://{}/maps/api/place/details/json", api_base);
    let key = env::var("GOOGLE_MAPS_API_KEY")?;

    let res = reqwest::Client::new()
        .get(url)
        .query(&[("key", key)])
        .query(&[("sessiontoken", session_token)])
        .query(&[("place_id", id)])
        .send()
        .await?;

    let status_code = res.status().as_u16();

    if status_code >= 500 {
        return Err(upstream_error());
    } else if status_code != 200 {
        return Err(invalid_input_error());
    }

    let data: Response<Place> = res.json().await?;

    if data.status != "OK" {
        return Err(invalid_input_error());
    }

    Ok(data.result.ok_or_else(|| unexpected_error())?)
}
