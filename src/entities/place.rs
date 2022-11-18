use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct Place {
    id: Uuid,
    latitude: f64,
    longitude: f64,
    description: String,
}
