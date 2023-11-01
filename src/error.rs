//! Error handling for the entire application.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

/// A specialized `Result` type for the application.
pub type Result<T> = std::result::Result<T, AppError>;

/// Custom error type.
pub struct AppError(anyhow::Error);

// Convert any `anyhow::Error` into `AppError`.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

// Convert `AppError` into an Axum response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // TODO: when in production don't return any message
        // TODO: log error? not sure if this is the right place for it
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}
