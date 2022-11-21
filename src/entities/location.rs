use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{invalid_input_error, Error};

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LocationSource {
    GooglePlaces {
        place_id: String,
        session_token: String,
    },
    Coordinates(Coordinates),
}

#[derive(Serialize, Deserialize)]
pub struct LocationToken {
    pub id: Uuid,
    pub location: Location,
}

#[derive(Serialize, Deserialize)]
pub struct Location {
    pub description: String,
    pub coordinates: Coordinates,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Coordinates {
    pub latitude: f64,
    pub longitude: f64,
}

impl Into<String> for Coordinates {
    fn into(self) -> String {
        format!("{}, {}", self.latitude, self.longitude)
    }
}

impl TryFrom<String> for Coordinates {
    type Error = Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut components = value.split(",");

        let lat = components.next().ok_or_else(|| invalid_input_error())?;
        let lng = components.next().ok_or_else(|| invalid_input_error())?;

        if components.next().is_some() {
            return Err(invalid_input_error());
        }

        let lat = lat.parse::<f64>().map_err(|_| invalid_input_error())?;
        let lng = lng.parse::<f64>().map_err(|_| invalid_input_error())?;

        Ok(Coordinates {
            latitude: lat,
            longitude: lng,
        })
    }
}