//! # `ValidatedJson` extractor
//!
//! A drop-in replacement for [`axum::Json`] that also runs
//! [`validator::Validate`] on the deserialized body. Invalid input is
//! rejected with `422 Unprocessable Entity` and a structured list of
//! field-level errors, before any handler code runs.
//!
//! ## Why a custom extractor?
//!
//! Without it, every handler that accepts a body would start with the same
//! three lines:
//!
//! ```ignore
//! let body = ...;
//! body.validate().map_err(|e| /* build a Response */)?;
//! ```
//!
//! That is repetitive, easy to forget, and produces inconsistent error
//! shapes across handlers. Extractors move validation into the type
//! system: a handler that declares `ValidatedJson<T>` **cannot** receive
//! an unvalidated `T`.
//!
//! ## Why no `#[async_trait]`?
//!
//! Axum 0.8's [`FromRequest`] trait uses native `async fn` (Rust 1.75+
//! `async fn in trait`). Wrapping our impl in `#[async_trait]` desugars it
//! into a `Pin<Box<dyn Future + 'async_trait>>` form that introduces an
//! extra lifetime — and that lifetime does not exist on the trait's
//! declaration, producing E0195. The correct pattern for Axum 0.8 is a
//! plain `async fn` inside `impl`, with no macro.
//!
//! The same rule applies to any custom extractor we write later
//! (`AuthClaims`, etc.). If you see E0195 on `from_request`, the first
//! thing to check is a stray `#[async_trait]`.
//!
//! ## Error shape
//!
//! - Malformed JSON or wrong `Content-Type` → `400 Bad Request` with
//!   `{"error": "<reason>"}`.
//! - Well-formed JSON but failed validation → `422 Unprocessable Entity`
//!   with `{"error": "validation failed", "details": [ ... ]}`. Every
//!   field error is included, not just the first — a client can fix all
//!   issues in one round trip.
//!
//! ## References
//!
//! - [`axum::extract::FromRequest`](https://docs.rs/axum/latest/axum/extract/trait.FromRequest.html)
//! - [`validator` crate](https://docs.rs/validator/latest/validator/)

use axum::extract::{FromRequest, Request};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::de::DeserializeOwned;
use serde_json::json;
use validator::Validate;

/// Wraps a JSON body, validating it on the way in.
pub struct ValidatedJson<T>(pub T);

impl<S, T> FromRequest<S> for ValidatedJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Validate,
{
    /// The error type this extractor returns. It must implement
    /// [`IntoResponse`], which [`Response`] does trivially — so we can
    /// build the rejection using the same tools we use for success.
    type Rejection = Response;

    async fn from_request(
        request: Request,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        // Step 1: hand the request to `Json`'s extractor. This does the
        // content-type check, body reading, and deserialization. If any
        // of those fail, we translate the rejection into our own error
        // shape so the API always returns the same JSON envelope.
        //
        // The turbofish `Json::<T>` is required because `T` is generic
        // here — the compiler cannot infer which `Json` we mean.
        let Json(value) = Json::<T>::from_request(request, state)
            .await
            .map_err(|rejection| {
                // `JsonRejection` implements `IntoResponse`, so it already
                // knows how to build a plausible response. We could use it
                // directly, but we want our own envelope: `{"error": ...}`.
                //
                // `rejection.body_text()` returns the human-readable
                // message. It is intended for diagnostics, not for
                // user-facing copy, but for a validation library it is
                // fine.
                let message = rejection.body_text();
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": message })),
                )
                    .into_response()
            })?;

        // Step 2: run the validator. On failure, collect *every* field
        // error rather than short-circuiting on the first. This gives the
        // client everything they need to fix in one pass.
        value.validate().map_err(|errors| {
            let mut details = Vec::new();

            // `errors.field_errors()` yields a map of
            // `field_name -> Vec<FieldError>`. We flatten it into a
            // stable list shape: `[{field, code, message}, ...]`.
            for (field, field_errors) in errors.field_errors() {
                for error in field_errors {
                    // `error.message` is `Option<Cow<str>>`. Fall back to
                    // the code when no message was supplied, so the client
                    // always sees *something* meaningful.
                    let message = error
                        .message
                        .as_ref()
                        .map(|m| m.to_string())
                        .unwrap_or_else(|| error.code.to_string());

                    details.push(json!({
                        "field": field,
                        "code": error.code,
                        "message": message,
                    }));
                }
            }

            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({
                    "error": "validation failed",
                    "details": details,
                })),
            )
                .into_response()
        })?;

        Ok(ValidatedJson(value))
    }
}