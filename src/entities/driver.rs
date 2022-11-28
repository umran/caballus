use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{invalid_invocation_error, Error};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Driver {
    pub id: Uuid,
    pub user_id: Uuid,
    pub status: Status,
    pub active_vehicle_id: Option<Uuid>,
    pub active_trip_id: Option<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "name", rename_all = "snake_case")]
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

    #[tracing::instrument]
    pub fn assign_trip(&mut self, trip_id: Uuid) -> Result<(), Error> {
        match self.status {
            Status::Available => {
                self.active_trip_id = Some(trip_id);
                Ok(())
            }
            _ => Err(invalid_invocation_error()),
        }
    }
}
