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
    pub fare_ceiling: f64,
    pub fare: Option<f64>,
    pub driver_id: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "name", rename_all = "snake_case")]
pub enum Status {
    Searching {
        deadline: DateTime<Utc>,
    },
    PendingConfirmation {
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
    pub fn new(passenger_id: Uuid, route: Route, fare_ceiling: f64) -> Self {
        let status = Status::Searching {
            deadline: Utc::now() + Duration::seconds(60),
        };

        Self {
            id: Uuid::new_v4(),
            status,
            passenger_id,
            route,
            fare_ceiling,
            fare: None,
            driver_id: None,
        }
    }

    pub fn status_string(&self) -> String {
        match self.status {
            Status::Searching { deadline: _ } => "SEARCHING".to_string(),
            Status::PendingConfirmation {
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
            Status::Searching { deadline: _ } => true,
            _ => false,
        }
    }

    #[tracing::instrument]
    pub fn search_deadline(&self) -> Result<DateTime<Utc>, Error> {
        match &self.status {
            Status::Searching { deadline } => Ok(*deadline),
            _ => Err(invalid_invocation_error()),
        }
    }

    #[tracing::instrument]
    pub fn cancel(&mut self, is_passenger: bool) -> Result<(), Error> {
        let penalty_bearer = self.cancellation_result(is_passenger)?;

        self.status = Status::Cancelled { penalty_bearer };
        Ok(())
    }

    #[tracing::instrument]
    fn cancellation_result(&self, is_passenger: bool) -> Result<Option<PenaltyBearer>, Error> {
        match &self.status {
            Status::Searching { deadline: _ }
            | Status::PendingConfirmation {
                deadline: _,
                driver_id: _,
                fare: _,
            } => Ok(None),
            Status::DriverEnRoute { deadline } => match is_passenger {
                true => {
                    if Utc::now() >= *deadline {
                        return Ok(Some(PenaltyBearer::Driver));
                    }

                    Ok(Some(PenaltyBearer::Passenger))
                }
                false => Ok(Some(PenaltyBearer::Driver)),
            },
            Status::DriverArrived { is_late, timestamp } => match is_passenger {
                true => {
                    if *is_late {
                        return Ok(Some(PenaltyBearer::Driver));
                    }

                    Ok(Some(PenaltyBearer::Passenger))
                }
                false => {
                    if !*is_late && Utc::now() >= (*timestamp).add(Duration::minutes(5)) {
                        return Ok(Some(PenaltyBearer::Passenger));
                    }

                    Ok(Some(PenaltyBearer::Driver))
                }
            },
            _ => Err(invalid_invocation_error()),
        }
    }
}
