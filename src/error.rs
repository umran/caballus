use std::fmt::Debug;
use tracing::instrument;

use crate::db::TransactionError;

#[derive(Debug)]
pub struct Error {
    pub code: i32,
    pub message: String,
}

impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Error {
        database_error(err)
    }
}

impl From<TransactionError<Error>> for Error {
    fn from(err: TransactionError<Error>) -> Error {
        match err {
            TransactionError::ApplicationError(err) => err,
            TransactionError::DBError(err) => database_error(err),
        }
    }
}

pub fn invalid_state_error() -> Error {
    Error {
        code: 100,
        message: "invalid state".to_string(),
    }
}

pub fn invalid_input_error() -> Error {
    Error {
        code: 101,
        message: "invalid input".to_string(),
    }
}

pub fn application_state_error() -> Error {
    Error {
        code: 1,
        message: "application state error".to_string(),
    }
}

#[instrument]
pub fn database_error<T: Debug>(err: T) -> Error {
    Error {
        code: 2,
        message: "database error".to_string(),
    }
}
