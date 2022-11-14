use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Driver {
    pub id: String,
    pub status: Status,
    pub active_vehicle_id: String,
}

#[derive(Serialize, Deserialize)]
pub enum Status {
    Assigned,
    Available,
}

impl Driver {
    pub fn status_string(&self) -> String {
        match self.status {
            Status::Assigned => "ASSIGNED".to_string(),
            Status::Available => "AVAILABLE".to_string(),
        }
    }
}
