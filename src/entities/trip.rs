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
    pub route: Route,
    pub passenger_id: Uuid,
    pub selected_bid_id: Option<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "name", rename_all = "snake_case")]
pub enum Status {
    Searching {
        deadline: DateTime<Utc>,
        radius: f64,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PenaltyBearer {
    Passenger,
    Driver,
}

impl Trip {
    pub fn new(passenger_id: Uuid, route: Route) -> Self {
        let status = Status::Searching {
            deadline: Utc::now() + Duration::seconds(60),
            radius: 1000.0,
        };

        Self {
            id: Uuid::new_v4(),
            status,
            route,
            passenger_id,
            selected_bid_id: None,
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

    pub fn search_deadline(&self) -> Result<DateTime<Utc>, Error> {
        match &self.status {
            Status::Searching {
                deadline,
                radius: _,
            } => Ok(*deadline),
            _ => Err(invalid_invocation_error()),
        }
    }

    pub fn expand_search(&mut self) -> Result<(), Error> {
        match &self.status {
            Status::Searching {
                deadline: _,
                radius,
            } => {
                self.status = Status::Searching {
                    deadline: Utc::now() + Duration::seconds(60),
                    radius: *radius * 1.1,
                };

                Ok(())
            }
            _ => Err(invalid_invocation_error()),
        }
    }

    pub fn select_bid(&mut self, bid_id: Uuid) -> Result<(), Error> {
        match &self.status {
            Status::Searching {
                deadline: _,
                radius: _,
            } => {
                self.selected_bid_id = Some(bid_id);

                self.status = Status::PendingConfirmation {
                    deadline: Utc::now() + Duration::seconds(60),
                };

                Ok(())
            }
            _ => Err(invalid_invocation_error()),
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
            _ => Err(invalid_invocation_error()),
        }
    }
}
