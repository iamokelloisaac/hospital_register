//! # Custom extractors
//!
//! Axum extractors that run *before* a handler and either produce a value or
//! reject the request with a specific status code.
//!
//! ## Planned extractors
//!
//! - `ValidatedJson<T>` — deserializes a JSON body and runs
//!   [`validator::Validate`] on it. On failure, returns `400 Bad Request`
//!   with a structured error listing every validation error, rather than the
//!   first one.
//! - `AuthClaims` — extracts and verifies a JWT from the `Authorization`
//!   header, producing the decoded claims.
//!
//! No extractors are implemented yet.
pub mod validated_json;