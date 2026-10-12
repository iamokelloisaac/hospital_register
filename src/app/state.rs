//! # Application state
//!
//! [`AppState`] is the value injected into every Axum handler and into
//! every middleware that needs shared resources. It is constructed exactly
//! once in [`crate::main`], wrapped in `Arc` fields, and cloned per request
//! by Axum.
//!
//! ## Why `Arc`?
//!
//! Axum clones the state for each request. Without `Arc`, every clone
//! would deep-copy the settings struct and the connection pools. Wrapping
//! each field in `Arc` makes cloning a pointer copy, which is essentially
//! free. `PgPool` and `ConnectionManager` are internally reference-counted
//! already, but wrapping them in `Arc` too keeps the pattern uniform.
//!
//! ## Wiring order
//!
//! Repositories are constructed first (they need only the `PgPool`).
//! Services are constructed next (they need repositories). The rate limit
//! service needs the whole `DbPools` because it uses Redis as well as
//! Postgres. The resulting `AppState` holds everything downstream code
//! could ask for.

use std::sync::Arc;

use crate::config::settings::Settings;
use crate::db::pool::DbPools;
use crate::db::repositories::appointment_repo::PostgresAppointmentRepository;
use crate::db::repositories::audit_repo::PostgresAuditLogRepository;
use crate::db::repositories::doctor_repo::PostgresDoctorRepository;
use crate::db::repositories::patient_repo::PostgresPatientRepository;
use crate::services::appointment_service::AppointmentService;
use crate::services::audit_service::AuditService;
use crate::services::auth_service::AuthService;
use crate::services::doctor_service::DoctorService;
use crate::services::patient_service::PatientService;
use crate::services::rate_limit_service::RateLimitService;

/// Shared state available to every handler and to middleware.
#[derive(Clone)]
pub struct AppState {
    /// Connection pools for Postgres and Redis.
    pub pools: Arc<DbPools>,

    /// Loaded configuration. Immutable after startup.
    pub settings: Arc<Settings>,

    /// Audit log service. Shared by every mutating service.
    pub audit_service: Arc<AuditService>,

    /// Authentication service: password verification and JWT issuance.
    pub auth_service: Arc<AuthService>,

    /// Patient business logic.
    pub patient_service: Arc<PatientService>,

    /// Doctor business logic.
    pub doctor_service: Arc<DoctorService>,

    /// Appointment business logic.
    pub appointment_service: Arc<AppointmentService>,

    /// Rate limiter middleware backend.
    pub rate_limit_service: Arc<RateLimitService>,
}

impl AppState {
    /// Constructs an `AppState`, wiring repositories and services together.
    ///
    /// This is the **only** place in the codebase where concrete
    /// implementations of repository traits are chosen. Everywhere else,
    /// services depend on the trait, not on the concrete struct. Swapping
    /// Postgres for another store would change only this file.
    pub fn new(pools: DbPools, settings: Settings) -> Self {
        // Clone the pool once. `PgPool` clones are cheap (Arc internally),
        // and we hand one to each repository so they remain independent.
        let pg = pools.pg.clone();

        // --- Repositories ---
        let patient_repository =
            Arc::new(PostgresPatientRepository::new(pg.clone()));
        let doctor_repository =
            Arc::new(PostgresDoctorRepository::new(pg.clone()));
        let appointment_repository =
            Arc::new(PostgresAppointmentRepository::new(pg.clone()));
        let audit_repository =
            Arc::new(PostgresAuditLogRepository::new(pg));

        // --- Services ---
        //
        // The audit service is created first because several other
        // services depend on it. `Arc::clone` is a pointer copy, not a
        // deep copy — safe to share freely.
        let audit_service = Arc::new(AuditService::new(audit_repository));

        let auth_service = Arc::new(AuthService::new(
            doctor_repository.clone(),
            settings.secrets.jwt_secret.clone(),
            settings.jwt.access_ttl_minutes,
        ));

        let patient_service = Arc::new(PatientService::new(
            patient_repository,
            audit_service.clone(),
        ));

        let doctor_service = Arc::new(DoctorService::new(
            doctor_repository,
            audit_service.clone(),
        ));

        let appointment_service = Arc::new(AppointmentService::new(
            appointment_repository,
            audit_service.clone(),
        ));

        // The rate limit service needs Redis, so it takes the whole pool
        // set. Wrap `DbPools` in an Arc *first*, then construct the service
        // with a clone. This avoids having to restructure `DbPools` later.
        let pools_arc = Arc::new(pools);
        let rate_limit_service = Arc::new(RateLimitService::new(
            pools_arc.clone(),
        ));

        Self {
            pools: pools_arc,
            settings: Arc::new(settings),
            audit_service,
            auth_service,
            patient_service,
            doctor_service,
            appointment_service,
            rate_limit_service,
        }
    }
}