use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entities::Route;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Quote {
    pub token: Uuid,
    pub route: Route,
    pub max_fare: f64,
}

impl Quote {
    pub fn new(route: Route, max_fare: f64) -> Self {
        Self {
            token: Uuid::new_v4(),
            route,
            max_fare,
        }
    }
}
