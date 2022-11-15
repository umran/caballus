use serde::{Deserialize, Serialize};

use crate::error::{self, Error};

#[derive(Serialize, Deserialize)]
pub struct Driver {
    pub id: String,
    pub status: Status,
    pub active_vehicle_id: Option<String>,
    pub active_trip_id: Option<String>,
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

    pub fn is_available(&self) -> bool {
        match self.status {
            Status::Available => true,
            _ => false,
        }
    }

    pub fn assign_trip(&mut self, trip_id: String) -> Result<(), Error> {
        match self.status {
            Status::Available => {
                self.active_trip_id = Some(trip_id);
                Ok(())
            }
            _ => Err(error::invalid_state_error()),
        }
    }
}
