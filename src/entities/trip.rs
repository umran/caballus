use std::ops::Add;

use chrono::{DateTime, Duration, Utc};
use oso::PolarClass;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entities::Route;
use crate::error::{invalid_invocation_error, Error};

#[derive(Clone, Debug, Serialize, Deserialize, PolarClass)]
pub struct Trip {
    #[polar(attribute)]
    pub id: Uuid,
    #[polar(attribute)]
    pub status: Status,
    #[polar(attribute)]
    pub passenger_id: Uuid,
    pub route: Route,
    pub max_fare: f64,
    pub fare: Option<f64>,
    #[polar(attribute)]
    pub driver_id: Option<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "name", rename_all = "snake_case")]
pub enum Status {
    Searching,
    PendingAssignment {
        deadline: DateTime<Utc>,
        driver_id: Uuid,
        fare: f64,
    },
    DriverEnRoute {
        deadline: DateTime<Utc>,
    },
    DriverArrived {
        is_late: bool,
        timestamp: DateTime<Utc>,
    },
    Cancelled {
        penalty_bearer: Option<PenaltyBearer>,
    },
    Completed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PenaltyBearer {
    Passenger,
    Driver,
}

impl Status {
    pub fn name(&self) -> String {
        match self {
            Self::Searching => "searching".into(),
            Self::PendingAssignment {
                deadline: _,
                driver_id: _,
                fare: _,
            } => "pending_assessment".into(),
            Self::DriverEnRoute { deadline: _ } => "driver_en_route".into(),
            Self::DriverArrived {
                is_late: _,
                timestamp: _,
            } => "driver_arrived".into(),
            Self::Cancelled { penalty_bearer: _ } => "cancelled".into(),
            Self::Completed => "completed".into(),
        }
    }
}

impl PolarClass for Status {
    fn get_polar_class_builder() -> oso::ClassBuilder<Status> {
        oso::Class::builder()
            .name("TripStatus")
            .add_attribute_getter("name", |recv: &Status| recv.name())
            .add_attribute_getter("driver_id", |recv: &Status| match recv {
                Status::PendingAssignment {
                    deadline: _,
                    driver_id,
                    fare: _,
                } => Some(driver_id.clone()),
                _ => None,
            })
    }

    fn get_polar_class() -> oso::Class {
        let builder = Status::get_polar_class_builder();
        builder.build()
    }
}

impl Trip {
    pub fn new(passenger_id: Uuid, route: Route, max_fare: f64) -> Self {
        let status = Status::Searching;

        Self {
            id: Uuid::new_v4(),
            status,
            passenger_id,
            route,
            max_fare,
            fare: None,
            driver_id: None,
        }
    }

    pub fn is_searching(&self) -> bool {
        match &self.status {
            Status::Searching => true,
            _ => false,
        }
    }

    #[tracing::instrument]
    pub fn request_driver(&mut self, driver_id: Uuid, fare: f64) -> Result<(), Error> {
        match self.status {
            Status::Searching => {
                self.status = Status::PendingAssignment {
                    deadline: Utc::now() + Duration::seconds(30),
                    driver_id,
                    fare,
                };
                Ok(())
            }
            _ => Err(invalid_invocation_error()),
        }
    }

    #[tracing::instrument]
    pub fn release_driver(&mut self) -> Result<Uuid, Error> {
        match self.status {
            Status::PendingAssignment {
                deadline: _,
                driver_id,
                fare: _,
            } => {
                self.status = Status::Searching;
                Ok(driver_id)
            }
            _ => Err(invalid_invocation_error()),
        }
    }

    #[tracing::instrument]
    pub fn assign_driver(&mut self) -> Result<Uuid, Error> {
        match self.status {
            Status::PendingAssignment {
                deadline: _,
                driver_id,
                fare,
            } => {
                self.status = Status::DriverEnRoute {
                    deadline: Utc::now() + Duration::minutes(15),
                };
                self.driver_id = Some(driver_id.clone());
                self.fare = Some(fare);

                Ok(driver_id)
            }
            _ => Err(invalid_invocation_error()),
        }
    }

    #[tracing::instrument]
    pub fn cancel(&mut self, is_passenger: bool) -> Result<Option<Uuid>, Error> {
        let (penalty_bearer, freed_driver_id) = self.cancellation_result(is_passenger)?;

        self.status = Status::Cancelled { penalty_bearer };
        Ok(freed_driver_id)
    }

    #[tracing::instrument]
    fn cancellation_result(
        &self,
        is_passenger: bool,
    ) -> Result<(Option<PenaltyBearer>, Option<Uuid>), Error> {
        match &self.status {
            Status::Searching => Ok((None, None)),
            Status::PendingAssignment {
                deadline: _,
                driver_id,
                fare: _,
            } => Ok((None, Some(driver_id.clone()))),
            Status::DriverEnRoute { deadline } => match is_passenger {
                true => {
                    if Utc::now() >= *deadline {
                        return Ok((Some(PenaltyBearer::Driver), self.driver_id.clone()));
                    }

                    Ok((Some(PenaltyBearer::Passenger), self.driver_id.clone()))
                }
                false => Ok((Some(PenaltyBearer::Driver), self.driver_id.clone())),
            },
            Status::DriverArrived { is_late, timestamp } => match is_passenger {
                true => {
                    if *is_late {
                        return Ok((Some(PenaltyBearer::Driver), self.driver_id.clone()));
                    }

                    Ok((Some(PenaltyBearer::Passenger), self.driver_id.clone()))
                }
                false => {
                    if !*is_late && Utc::now() >= (*timestamp).add(Duration::minutes(5)) {
                        return Ok((Some(PenaltyBearer::Passenger), self.driver_id.clone()));
                    }

                    Ok((Some(PenaltyBearer::Driver), self.driver_id.clone()))
                }
            },
            _ => Err(invalid_invocation_error()),
        }
    }
}
