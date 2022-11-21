use axum::extract::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use std::env;
use std::fmt::Debug;

#[derive(Debug)]
pub struct Error {
    pub code: i32,
    pub message: String,
}

impl From<env::VarError> for Error {
    fn from(err: env::VarError) -> Self {
        env_var_error(err)
    }
}

impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        database_error(err)
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        reqwest_error(err)
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, error_message) = match self.code {
            1..=99 => (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error"),
            _ => (StatusCode::BAD_REQUEST, self.message.as_str()),
        };

        let body = Json(json!({
            "code": self.code,
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

pub fn invalid_state_error() -> Error {
    Error {
        code: 100,
        message: "invalid state".into(),
    }
}

pub fn invalid_input_error() -> Error {
    Error {
        code: 101,
        message: "invalid input".into(),
    }
}

pub fn unimplemented_error() -> Error {
    Error {
        code: 0,
        message: "unimplemented error".into(),
    }
}

pub fn env_var_error(_: env::VarError) -> Error {
    Error {
        code: 1,
        message: "environment variable error".into(),
    }
}

pub fn database_error<T: Debug>(_: T) -> Error {
    Error {
        code: 2,
        message: "database error".into(),
    }
}

pub fn reqwest_error(_: reqwest::Error) -> Error {
    Error {
        code: 3,
        message: "reqwest error".into(),
    }
}

pub fn upstream_error() -> Error {
    Error {
        code: 4,
        message: "upstream error".into(),
    }
}

pub fn unexpected_error() -> Error {
    Error {
        code: 5,
        message: "unexpected error".into(),
    }
}
