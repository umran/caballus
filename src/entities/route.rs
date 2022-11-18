use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entities::Place;

#[derive(Serialize, Deserialize)]
pub struct Route {
    pub id: String,
    pub origin: Place,
    pub destination: Place,
    // ...
}

impl Route {
    pub fn new(origin: Place, destination: Place) -> Self {
        Route {
            id: Uuid::new_v4().to_string(),
            origin,
            destination,
        }
    }
}
