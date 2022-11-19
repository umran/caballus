use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entities::Location;

#[derive(Serialize, Deserialize)]
pub struct Route {
    pub id: String,
    pub origin: Location,
    pub destination: Location,
    // ...
}

impl Route {
    pub fn new(origin: Location, destination: Location) -> Self {
        Route {
            id: Uuid::new_v4().to_string(),
            origin,
            destination,
        }
    }
}
