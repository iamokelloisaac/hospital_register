//! # Repository traits
//!
//! The **contract** between the service layer and the database layer.
//!
//! Services depend on these traits, never on concrete implementations.
//! This inverts the usual dependency direction: instead of services knowing
//! about Postgres, Postgres is plugged into services behind an interface.
//!
//! ## Why `#[async_trait]`?
//!
//! Rust's stable `async fn` in traits does not yet support dynamic dispatch
//! (`dyn Trait`) or object safety. The [`async_trait`] crate papers over this
//! by desugaring the async methods into methods returning `Pin<Box<dyn Future>>`.
//! It is the standard approach until native async traits are stabilized
//! for these use cases.
//!
//! ## Why `Send + Sync`?
//!
//! Axum handlers run inside a `tokio::task`, which may migrate between
//! threads. For a `Box<dyn DoctorRepository>` to be safely stored in
//! `AppState` and used from any handler, it must be `Send + Sync`.

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use crate::core::errors::CoreError;
use crate::core::models::appointment::{Appointment, AppointmentStatus};
use crate::core::models::audit_log::AuditLog;
use crate::core::models::doctor::Doctor;
use crate::core::models::patient::{BloodType, Patient};

// ---------------------------------------------------------------------------
// Patient
// ---------------------------------------------------------------------------

/// Persistence operations for [`Patient`].
#[async_trait]
pub trait PatientRepository: Send + Sync {
    async fn insert(
        &self,
        nin: &str,
        first_name: &str,
        last_name: &str,
        date_of_birth: NaiveDate,
        blood_type: Option<BloodType>,
    ) -> Result<Patient, CoreError>;

    async fn find_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<Patient>, CoreError>;

    async fn list(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Patient>, CoreError>;

    async fn update(
        &self,
        id: Uuid,
        first_name: Option<&str>,
        last_name: Option<&str>,
        blood_type: Option<BloodType>,
    ) -> Result<Patient, CoreError>;

    async fn soft_delete(&self, id: Uuid) -> Result<(), CoreError>;
}

// ---------------------------------------------------------------------------
// Doctor
// ---------------------------------------------------------------------------

/// Persistence operations for [`Doctor`].
///
/// Note the split between two read paths:
///
/// - `find_by_id`, `list` return [`Doctor`] — no hash, safe for responses.
/// - `find_by_email_with_hash` returns [`DoctorWithHash`] — used only by
///   the authentication flow. Keeping these separate means a hash can never
///   leak into a response by accident.
#[async_trait]
pub trait DoctorRepository: Send + Sync {
    /// Inserts a new doctor. `password_hash` must already be hashed.
    async fn insert(
        &self,
        email: &str,
        password_hash: &str,
        full_name: &str,
        specialty: &str,
        role: &str,
    ) -> Result<Doctor, CoreError>;

    /// Looks up an active doctor by ID.
    async fn find_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<Doctor>, CoreError>;

    /// Looks up a doctor by email, returning the password hash alongside
    /// the doctor record so the auth service can verify credentials.
    async fn find_by_email_with_hash(
        &self,
        email: &str,
    ) -> Result<Option<DoctorWithHash>, CoreError>;

    /// Lists active doctors with pagination.
    async fn list(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Doctor>, CoreError>;

    /// Updates mutable profile fields on a doctor.
    async fn update(
        &self,
        id: Uuid,
        full_name: Option<&str>,
        specialty: Option<&str>,
    ) -> Result<Doctor, CoreError>;

    /// Soft-deletes a doctor by setting `deleted_at`.
    async fn soft_delete(&self, id: Uuid) -> Result<(), CoreError>;
}

/// Doctor record plus its password hash. Only the auth path needs this.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DoctorWithHash {
    pub id: Uuid,
    pub email: String,
    pub full_name: String,
    pub role: String,
    pub password_hash: String,
}

// ---------------------------------------------------------------------------
// Appointment
// ---------------------------------------------------------------------------

/// Persistence operations for [`Appointment`].
#[async_trait]
pub trait AppointmentRepository: Send + Sync {
    /// Inserts a new appointment.
    async fn insert(
        &self,
        patient_id: Uuid,
        doctor_id: Uuid,
        scheduled_for: DateTime<Utc>,
        notes: Option<&str>,
    ) -> Result<Appointment, CoreError>;

    /// Looks up an appointment by ID.
    async fn find_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<Appointment>, CoreError>;

    /// Lists appointments for a patient, newest scheduled first.
    async fn list_for_patient(
        &self,
        patient_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Appointment>, CoreError>;

    /// Lists upcoming appointments for a doctor.
    async fn list_for_doctor(
        &self,
        doctor_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Appointment>, CoreError>;

    /// Updates the status of an appointment.
    async fn update_status(
        &self,
        id: Uuid,
        status: AppointmentStatus,
    ) -> Result<Appointment, CoreError>;

    /// Returns true if the doctor already has an active (non-cancelled,
    /// non-no-show) appointment within `window_minutes` of `scheduled_for`.
    ///
    /// Used to prevent double-booking at the service layer.
    async fn has_conflict(
        &self,
        doctor_id: Uuid,
        scheduled_for: DateTime<Utc>,
        window_minutes: i64,
    ) -> Result<bool, CoreError>;
}

// ---------------------------------------------------------------------------
// Audit log
// ---------------------------------------------------------------------------

/// Persistence operations for the audit log.
#[async_trait]
pub trait AuditLogRepository: Send + Sync {
    /// Appends an audit event. Audit logs are append-only.
    async fn append(
        &self,
        actor_id: Option<Uuid>,
        action: &str,
        entity_type: &str,
        entity_id: Uuid,
        metadata: Option<serde_json::Value>,
    ) -> Result<AuditLog, CoreError>;

    /// Returns recent audit entries for a given entity, newest first.
    async fn list_for_entity(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        limit: i64,
    ) -> Result<Vec<AuditLog>, CoreError>;
}