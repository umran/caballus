use axum::extract::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use std::fmt::Debug;

#[derive(Debug)]
pub struct Error {
    pub code: i32,
    pub message: String,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, error_message) = match self.code {
            1..=99 => (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error"),
            _ => (StatusCode::BAD_REQUEST, self.message.as_str()),
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Error {
        database_error(err)
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

pub fn database_error<T: Debug>(_: T) -> Error {
    Error {
        code: 1,
        message: "database error".to_string(),
    }
}
