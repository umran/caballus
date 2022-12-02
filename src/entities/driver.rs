use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{invalid_invocation_error, Error};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Driver {
    pub id: Uuid,
    pub status: Status,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "name", rename_all = "snake_case")]
pub enum Status {
    Idle,
    Available,
    Requested { trip_id: Uuid },
    Assigned { trip_id: Uuid },
}

impl Driver {
    pub fn new(user_id: Uuid) -> Self {
        Self {
            id: user_id,
            status: Status::Idle,
        }
    }

    pub fn status_string(&self) -> String {
        match self.status {
            Status::Idle => "IDLE".into(),
            Status::Available => "AVAILABLE".into(),
            Status::Requested { trip_id: _ } => "REQUESTED".into(),
            Status::Assigned { trip_id: _ } => "ASSIGNED".into(),
        }
    }

    pub fn is_available(&self) -> bool {
        match self.status {
            Status::Available => true,
            _ => false,
        }
    }

    #[tracing::instrument]
    pub fn request(&mut self, trip_id: Uuid) -> Result<(), Error> {
        match self.status {
            Status::Available => {
                self.status = Status::Requested { trip_id };
                Ok(())
            }
            _ => Err(invalid_invocation_error()),
        }
    }

    pub fn assign(&mut self) -> Result<(), Error> {
        match self.status {
            Status::Requested { trip_id } => {
                self.status = Status::Assigned { trip_id };
                Ok(())
            }
            _ => Err(invalid_invocation_error()),
        }
    }

    #[tracing::instrument]
    pub fn free(&mut self) -> Result<(), Error> {
        match self.status {
            Status::Requested { trip_id: _ } | Status::Assigned { trip_id: _ } => {
                self.status = Status::Available;
            }
            _ => (),
        };

        Ok(())
    }

    #[tracing::instrument]
    pub fn start(&mut self) -> Result<(), Error> {
        match self.status {
            Status::Idle => {
                self.status = Status::Available;
            }
            _ => (),
        };

        Ok(())
    }

    #[tracing::instrument]
    pub fn stop(&mut self) -> Result<(), Error> {
        match self.status {
            Status::Available => {
                self.status = Status::Idle;
                Ok(())
            }
            _ => Err(invalid_invocation_error()),
        }
    }
}
