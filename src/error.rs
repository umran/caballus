use std::fmt::Debug;
use tracing::instrument;

#[derive(Debug)]
pub struct Error {
    pub code: i32,
    pub message: String,
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
