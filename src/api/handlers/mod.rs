//! # Handlers
//!
//! Async functions that process HTTP requests. Each handler is registered
//! on a route in [`crate::api::routes`].
//!
//! ## Handler contract
//!
//! A handler is any async function whose arguments implement
//! [`axum::extract::FromRequestParts`] or [`axum::extract::FromRequest`],
//! and whose return type implements [`axum::response::IntoResponse`].
//!
//! The canonical handler signature in this codebase is:
//!
//! ```ignore
//! async fn handler(
//!     State(state): State<AppState>,
//!     Extension(claims): Extension<Claims>,
//!     Path(id): Path<Uuid>,
//!     Query(params): Query<SomeQuery>,
//!     ValidatedJson(body): ValidatedJson<SomeRequest>,
//! ) -> Result<Json<SomeResponse>, AppError> {
//!     // delegate to a service, return the result
//! }
//! ```
//!
//! Note that `State` always comes first, and body-consuming extractors
//! (`ValidatedJson`, `Json`, `Form`) must come last.
//!
//! ## What a handler must **not** do
//!
//! - **No SQL.** That is the repository layer's job.
//! - **No business rules.** That is the service layer's job. Handlers
//!   extract, delegate, and serialize. Nothing more.
//! - **No authorization policy decisions** beyond the trivially obvious.
//!   We do currently inline a few `require_role` checks, but the goal is
//!   to move those into a Tower middleware layer so the policy lives in
//!   one place rather than scattered across handlers.
//!
//! ## Why separate files per resource?
//!
//! RustRover treats each `pub mod` declaration as a navigable node in the
//! project tree, and `Ctrl+Click` on a handler name in `routes.rs` jumps
//! directly to the correct file. A single `handlers.rs` with everything in
//! it would work, but navigation would be slower and the file would grow
//! faster than reviewers can comfortably read.

pub mod appointment_handler;
pub mod auth_handler;
pub mod doctor_handler;
pub mod health_handler;
pub mod patient_handler;
pub mod audit_handler;