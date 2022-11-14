use std::ops::Add;

use crate::error::{invalid_state_error, Error};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Trip {
    pub id: String,
    pub status: Status,
    pub route_id: String,
    pub passenger_id: String,
    pub accepted_bid_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub enum Status {
    Searching {
        deadline: DateTime<Utc>,
        radius: Radius,
    },
    PendingConfirmation {
        deadline: DateTime<Utc>,
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

#[derive(Serialize, Deserialize)]
pub enum PenaltyBearer {
    Passenger,
    Driver,
}

#[derive(Serialize, Deserialize)]
pub struct Radius(f64);

impl Trip {
    pub fn new(id: String, route_id: String, passenger_id: String) -> Self {
        let status = Status::Searching {
            deadline: Utc::now() + Duration::seconds(60),
            radius: Radius(1000.0),
        };

        Self {
            id,
            status,
            route_id,
            passenger_id,
            accepted_bid_id: None,
        }
    }

    pub fn status_string(&self) -> String {
        match self.status {
            Status::Searching {
                deadline: _,
                radius: _,
            } => "SEARCHING".to_string(),
            Status::PendingConfirmation { deadline: _ } => "PENDING_CONFIRMATION".to_string(),
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
            Status::Searching {
                deadline: _,
                radius: _,
            } => true,
            _ => false,
        }
    }

    pub fn expand_search(&mut self, radius: Radius) -> Result<(), Error> {
        match &self.status {
            Status::Searching {
                deadline: _,
                radius: _,
            } => {
                self.status = Status::Searching {
                    deadline: Utc::now() + Duration::seconds(60),
                    radius,
                };

                Ok(())
            }
            _ => Err(invalid_state_error()),
        }
    }

    pub fn accept_bid(&mut self, bid_id: String) -> Result<(), Error> {
        match &self.status {
            Status::Searching {
                deadline: _,
                radius: _,
            } => {
                self.accepted_bid_id = Some(bid_id);

                self.status = Status::PendingConfirmation {
                    deadline: Utc::now() + Duration::seconds(60),
                };

                Ok(())
            }
            _ => Err(invalid_state_error()),
        }
    }

    pub fn cancel(&mut self, is_passenger: bool) -> Result<(), Error> {
        let penalty_bearer = self.cancellation_result(is_passenger)?;

        self.status = Status::Cancelled { penalty_bearer };
        Ok(())
    }

    fn cancellation_result(&self, is_passenger: bool) -> Result<Option<PenaltyBearer>, Error> {
        match &self.status {
            Status::Searching {
                deadline: _,
                radius: _,
            }
            | Status::PendingConfirmation { deadline: _ } => Ok(None),
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
            _ => Err(invalid_state_error()),
        }
    }
}
