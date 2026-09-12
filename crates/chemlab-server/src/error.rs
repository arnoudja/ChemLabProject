use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chemlab_contracts::ErrorResponse;
use chemlab_db::DbError;

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
}

impl ApiError {
    pub fn bad_request(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code,
            message: message.into(),
        }
    }

    pub fn unauthorized(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code,
            message: message.into(),
        }
    }

    pub fn conflict(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            code,
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "internal",
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = ErrorResponse {
            error: self.message,
            code: self.code.to_string(),
        };
        (self.status, Json(body)).into_response()
    }
}

impl From<DbError> for ApiError {
    fn from(value: DbError) -> Self {
        match value {
            DbError::EmailTaken => {
                Self::conflict("email_taken", "That email is already registered")
            }
            DbError::UserNotFound => {
                Self::unauthorized("invalid_credentials", "Invalid email or password")
            }
            DbError::SessionInvalid => {
                Self::unauthorized("session_invalid", "Session expired or missing")
            }
            DbError::Sqlx(err) => {
                tracing::error!(error = %err, "database error");
                Self::internal("Database error")
            }
            DbError::Migrate(err) => {
                tracing::error!(error = %err, "migration error");
                Self::internal("Database migration error")
            }
        }
    }
}
