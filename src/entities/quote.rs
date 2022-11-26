use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entities::Route;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Quote {
    pub token: Uuid,
    pub route: Route,
    pub amount: f64,
}

impl Quote {
    pub fn new(route: Route, amount: f64) -> Self {
        Self {
            token: Uuid::new_v4(),
            route,
            amount,
        }
    }
}
