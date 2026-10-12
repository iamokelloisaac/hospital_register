//! # Application assembly
//!
//! Wires the layers together.
//!
//! - [`state`] — the shared `AppState` injected into every handler.
//! - [`router`] — builds the top-level Axum router.
//!
//! This module is the only place that knows about *all* layers. It is the
//! composition root.

pub mod router;
pub mod state;