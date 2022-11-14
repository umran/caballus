use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Route {
    pub id: String,
    pub origin: Place,
    pub destination: Place,
    pub distance: f64,
    // ...
}

#[derive(Serialize, Deserialize)]
pub struct Place {
    latitude: f64,
    longitude: f64,
    description: String,
}
