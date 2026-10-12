//! # Routes
//!
//! Route definitions. A "route" is a mapping from an HTTP method + path
//! pattern to a handler.
//!
//! Routes are organized by API version:
//!
//! - [`v1`] — the `/api/v1` subtree.
//!
//! Unversioned operational endpoints (`/health`, `/metrics`) are registered
//! directly on the top-level router in [`crate::app::router`], because they
//! are not part of the versioned business API.

pub mod appointments;
pub mod audit;
pub mod auth;
pub mod doctors;
pub mod patients;
pub mod v1;