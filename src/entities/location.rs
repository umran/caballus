use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct LocationToken {
    id: Uuid,
    location: Location,
}

#[derive(Serialize, Deserialize)]
pub struct Location {
    description: String,
    coordinates: Coordinates,
}

#[derive(Serialize, Deserialize)]
pub struct Coordinates {
    pub latitude: f64,
    pub longitude: f64,
}
