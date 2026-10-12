//! # API Layer
//!
//! The HTTP boundary. Everything Axum-specific lives here.
//!
//! - [`routes`] — route definitions and router composition.
//! - [`handlers`] — async functions that process requests.
//! - [`extractors`] — custom [`axum::extract::FromRequest`] implementations.
//! - [`middleware`] — Tower layers (auth, rate limiting).
//! - [`dto`] — request and response body shapes.
//! - [`error`] — translates [`CoreError`](crate::core::errors::CoreError)
//!   into HTTP responses.
//!
//! ## Rule
//!
//! Handlers do three things and nothing more:
//!
//! 1. **Extract** input from the request.
//! 2. **Delegate** to a service.
//! 3. **Serialize** the result into a response.
//!
//! Any logic beyond that belongs in a service.

pub mod dto;
pub mod error;
pub mod extractors;
pub mod handlers;
pub mod middleware;
pub mod routes;