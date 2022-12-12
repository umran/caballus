use oso::PolarClass;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{invalid_invocation_error, Error};

#[derive(Clone, Debug, Serialize, Deserialize, PolarClass)]
pub struct Passenger {
    #[polar(attribute)]
    pub id: Uuid,
    pub status: Status,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "name", rename_all = "snake_case")]
pub enum Status {
    Inactive,
    Active { trip_id: Uuid },
}

impl Status {
    pub fn name(&self) -> String {
        match self {
            Self::Inactive => "inactive".into(),
            Self::Active { trip_id: _ } => "active".into(),
        }
    }
}

impl Passenger {
    pub fn is_active(&self) -> bool {
        match self.status {
            Status::Active { trip_id: _ } => true,
            _ => false,
        }
    }

    pub fn activate(&mut self, trip_id: Uuid) -> Result<(), Error> {
        match self.status {
            Status::Inactive => {
                self.status = Status::Active { trip_id };
                Ok(())
            }
            _ => Err(invalid_invocation_error()),
        }
    }

    pub fn deactivate(&mut self) -> Result<(), Error> {
        match self.status {
            Status::Active { trip_id: _ } => {
                self.status = Status::Inactive;
                Ok(())
            }
            _ => Err(invalid_invocation_error()),
        }
    }
}
