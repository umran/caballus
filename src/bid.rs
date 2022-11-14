use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Bid {
    pub id: String,
    pub trip_id: String,
    pub driver_id: String,
    pub amount: u64,
}
