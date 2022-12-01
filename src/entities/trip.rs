use std::ops::Add;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entities::Route;
use crate::error::{invalid_invocation_error, Error};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Trip {
    pub id: Uuid,
    pub status: Status,
    pub passenger_id: Uuid,
    pub route: Route,
    pub max_fare: f64,
    pub fare: Option<f64>,
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

    pub fn status_string(&self) -> String {
        match self.status {
            Status::Searching => "SEARCHING".to_string(),
            Status::PendingAssignment {
                deadline: _,
                driver_id: _,
                fare: _,
            } => "PENDING_CONFIRMATION".to_string(),
            Status::DriverEnRoute { deadline: _ } => "DRIVER_EN_ROUTE".to_string(),
            Status::DriverArrived {
                is_late: _,
                timestamp: _,
            } => "DRIVER_ARRIVED".to_string(),
            Status::Cancelled { penalty_bearer: _ } => "CANCELLED".to_string(),
            Status::Completed => "COMPLETED".to_string(),
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
    pub fn derequest_driver(&mut self) -> Result<Uuid, Error> {
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
    pub fn assign_driver(&mut self) -> Result<(), Error> {
        match self.status {
            Status::PendingAssignment {
                deadline: _,
                driver_id,
                fare,
            } => {
                self.status = Status::DriverEnRoute {
                    deadline: Utc::now() + Duration::minutes(15),
                };
                self.driver_id = Some(driver_id);
                self.fare = Some(fare);

                Ok(())
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
