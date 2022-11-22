use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bid {
    pub id: Uuid,
    pub trip_id: Uuid,
    pub driver_id: Uuid,
    pub amount: i64,
}

impl Bid {
    pub fn new(trip_id: Uuid, driver_id: Uuid, amount: i64) -> Self {
        Bid {
            id: Uuid::new_v4(),
            trip_id,
            driver_id,
            amount,
        }
    }
}
