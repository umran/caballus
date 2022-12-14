use geo_types::{Geometry, Point};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::Error;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LocationSource {
    GooglePlaces {
        place_id: String,
        session_token: String,
    },
    Coordinates(Coordinates),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Location {
    pub token: Uuid,
    pub coordinates: Coordinates,
    pub description: String,
}

impl Location {
    pub fn new(coordinates: Coordinates, description: String) -> Self {
        Self {
            token: Uuid::new_v4(),
            coordinates,
            description,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Coordinates {
    pub lat: f64,
    pub lng: f64,
}

impl Into<String> for Coordinates {
    fn into(self) -> String {
        format!("{}, {}", self.lat, self.lng)
    }
}

impl TryFrom<String> for Coordinates {
    type Error = Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut components = value.split(",");

        let lat = components
            .next()
            .ok_or_else(|| Error::invalid_input_error())?;
        let lng = components
            .next()
            .ok_or_else(|| Error::invalid_input_error())?;

        if components.next().is_some() {
            return Err(Error::invalid_input_error());
        }

        let lat = lat
            .parse::<f64>()
            .map_err(|_| Error::invalid_input_error())?;
        let lng = lng
            .parse::<f64>()
            .map_err(|_| Error::invalid_input_error())?;

        Ok(Coordinates { lat, lng })
    }
}

impl Into<Point<f64>> for Coordinates {
    fn into(self) -> Point<f64> {
        Point::new(self.lat, self.lng)
    }
}

impl Into<Geometry<f64>> for Coordinates {
    fn into(self) -> Geometry<f64> {
        Geometry::Point(self.into())
    }
}
