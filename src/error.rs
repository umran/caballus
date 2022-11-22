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
        sqlx_error(err)
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        reqwest_error(err)
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, message) = match self.code {
            0..=99 => (StatusCode::INTERNAL_SERVER_ERROR, "internal server error"),
            _ => (StatusCode::BAD_REQUEST, self.message.as_str()),
        };

        let body = Json(json!({
            "code": self.code,
            "message": message,
        }));

        (status, body).into_response()
    }
}

pub fn sqlx_error(err: sqlx::Error) -> Error {
    tracing::error!("sqlx error: {:?}", err);

    Error {
        code: 2,
        message: "database error".into(),
    }
}

pub fn reqwest_error(err: reqwest::Error) -> Error {
    tracing::error!("reqwest error: {:?}", err);

    Error {
        code: 3,
        message: "reqwest error".into(),
    }
}

pub fn env_var_error(err: env::VarError) -> Error {
    tracing::warn!("env var error: {:?}", err);

    Error {
        code: 4,
        message: "environment variable error".into(),
    }
}

pub fn upstream_error() -> Error {
    tracing::warn!("upstream error");

    Error {
        code: 5,
        message: "upstream error".into(),
    }
}

pub fn invalid_invocation_error() -> Error {
    tracing::info!("invalid invocation error");

    Error {
        code: 100,
        message: "invalid invocation error".into(),
    }
}

pub fn invalid_input_error() -> Error {
    tracing::info!("invalid input error");

    Error {
        code: 101,
        message: "invalid input error".into(),
    }
}
