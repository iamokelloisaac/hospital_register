//! # HTTP error mapping
//!
//! Translates [`CoreError`] into an HTTP response.
//!
//! ## Why a wrapper?
//!
//! `CoreError` is defined in the domain layer and knows nothing about HTTP.
//! `AppError` wraps it and implements [`axum::response::IntoResponse`], which
//! is what Axum needs to turn a returned error into a response. This keeps
//! the mapping in one place: change it here, and every handler changes.
//!
//! ## Internal errors
//!
//! [`CoreError::Internal`] carries a message intended for logs. The HTTP
//! response deliberately replaces it with a generic `"internal server error"`
//! so that database schema details, file paths, or stack traces never leak
//! to clients.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

use crate::core::errors::CoreError;

/// A newtype wrapper around [`CoreError`] that Axum can render.
pub struct AppError(pub CoreError);

impl From<CoreError> for AppError {
    fn from(error: CoreError) -> Self {
        AppError(error)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // Map each domain error to a status code and a client-safe message.
        let (status, message) = match &self.0 {
            CoreError::Validation(message) => {
                (StatusCode::BAD_REQUEST, message.clone())
            }
            CoreError::NotFound(message) => {
                (StatusCode::NOT_FOUND, message.clone())
            }
            CoreError::Conflict(message) => {
                (StatusCode::CONFLICT, message.clone())
            }
            CoreError::Unauthorized(message) => {
                (StatusCode::UNAUTHORIZED, message.clone())
            }
            CoreError::Forbidden(message) => {
                (StatusCode::FORBIDDEN, message.clone())
            }
            CoreError::Internal(message) => {
                // Log the real message. Never send it to the client.
                tracing::error!("internal error: {message}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_string(),
                )
            }
        };

        let body = Json(json!({ "error": message }));
        (status, body).into_response()
    }
}