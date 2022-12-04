use oso::PolarClass;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{invalid_invocation_error, Error};

#[derive(Clone, Debug, Serialize, Deserialize, PolarClass)]
pub struct Driver {
    #[polar(attribute)]
    pub id: Uuid,
    #[polar(attribute)]
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

impl Status {
    pub fn name(&self) -> String {
        match self {
            Status::Idle => "IDLE".into(),
            Status::Available => "AVAILABLE".into(),
            Status::Requested { trip_id: _ } => "REQUESTED".into(),
            Status::Assigned { trip_id: _ } => "ASSIGNED".into(),
        }
    }
}

impl PolarClass for Status {
    fn get_polar_class_builder() -> oso::ClassBuilder<Status> {
        oso::Class::builder()
            .name("DriverStatus")
            .add_attribute_getter("name", |recv: &Status| recv.name())
            .add_attribute_getter("trip_id", |recv: &Status| match recv {
                Status::Requested { trip_id } | Status::Assigned { trip_id } => {
                    Some(trip_id.clone())
                }
                _ => None,
            })
    }

    fn get_polar_class() -> oso::Class {
        let builder = Status::get_polar_class_builder();
        builder.build()
    }
}

impl Driver {
    pub fn new(user_id: Uuid) -> Self {
        Self {
            id: user_id,
            status: Status::Idle,
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
