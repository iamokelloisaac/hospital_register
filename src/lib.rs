//! # hospital_register
//!
//! A production-grade Hospital Register API built with Axum, SQLx, and PostgreSQL.
//!
//! ## Architecture
//!
//! This crate follows a **layered architecture**. Each layer has a single
//! responsibility, and dependencies flow in one direction only:
//!
//! ```text
//!   ┌─────────────────────────────────────────────────────┐
//!   │                      API Layer                      │
//!   │   (Axum routers, handlers, extractors, middleware)  │
//!   └──────────────────────┬──────────────────────────────┘
//!                          │ calls
//!                          ▼
//!   ┌─────────────────────────────────────────────────────┐
//!   │                    Service Layer                    │
//!   │         (business logic, orchestration)             │
//!   └──────────────────────┬──────────────────────────────┘
//!                          │ uses
//!                          ▼
//!   ┌─────────────────────────────────────────────────────┐
//!   │                   Domain Layer                      │
//!   │     (pure types, traits — no HTTP, no SQL)          │
//!   └──────────────────────┬──────────────────────────────┘
//!                          │ implemented by
//!                          ▼
//!   ┌─────────────────────────────────────────────────────┐
//!   │                 Database Layer                      │
//!   │        (SQLx repositories, Redis access)            │
//!   └─────────────────────────────────────────────────────┘
//! ```
//!
//! The `shared` module sits to the side, providing cross-cutting utilities
//! (telemetry, crypto) that any layer may use.
//!
//! ## Why a single crate?
//!
//! We deliberately chose a **single-crate monolith** over a workspace of
//! multiple crates. The reasons:
//!
//! - **Faster compile times.** A workspace recompiles dependent crates on
//!   every change; a single crate lets Cargo incrementally compile only the
//!   modules that changed.
//! - **Simpler navigation.** RustRover's module tree maps directly to the
//!   filesystem, and `Ctrl+Click` works across all layers without crate
//!   boundaries.
//! - **One `Cargo.toml`.** No version-skew between crates, no duplicated
//!   dependency declarations.
//!
//! The discipline that normally comes from crate boundaries is enforced by
//! **module boundaries** instead: `core` never imports `db`, `db` never
//! imports `api`, and so on. The compiler does not prevent cross-layer
//! imports, so this discipline is maintained by convention and code review.

pub mod api;
pub mod app;
pub mod config;
pub mod core;
pub mod db;
pub mod services;
pub mod shared;