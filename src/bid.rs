use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct Bid {
    pub id: String,
    pub trip_id: String,
    pub driver_id: String,
    pub amount: u64,
}

impl Bid {
    pub fn new(trip_id: String, driver_id: String, amount: u64) -> Self {
        Bid {
            id: Uuid::new_v4().to_string(),
            trip_id,
            driver_id,
            amount,
        }
    }
}
