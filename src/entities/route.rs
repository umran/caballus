use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::entities::Location;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Route {
    pub token: Uuid,
    pub origin: Location,
    pub destination: Location,
    pub directions: Value,
    // ...
}

impl Route {
    pub fn new(origin: Location, destination: Location, directions: Value) -> Self {
        Route {
            token: Uuid::new_v4(),
            origin,
            destination,
            directions,
        }
    }
}
