//! # Appointment repository (Postgres)
//!
//! Implements [`AppointmentRepository`] using SQLx.
//!
//! ## Status serialization
//!
//! `AppointmentStatus` is mapped to `TEXT` in Postgres with a `CHECK`
//! constraint (see the migration). We convert between the enum and the
//! canonical string form with two small helpers at the bottom of this file.
//! Keeping the conversion local means the domain enum stays free of
//! database concerns.
//!
//! ## Conflict detection
//!
//! [`has_conflict`](AppointmentRepository::has_conflict) is the guard
//! against double-booking. It is deliberately a repository method rather
//! than a service method because the query needs to know about the doctor's
//! schedule index — that is a persistence concern, not a domain one.
//!
//! Note that `has_conflict` is a *check*, not a *lock*. Under concurrent
//! load two bookings could both pass the check before either inserts. Real
//! serialization requires either a serializable transaction or an
//! exclusion constraint (Postgres `EXCLUDE USING gist` with a time range
//! type). We will add that constraint in a later migration; today we
//! establish the check-then-insert pattern that the constraint will
//! strengthen.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::core::errors::CoreError;
use crate::core::models::appointment::{Appointment, AppointmentStatus};
use crate::core::traits::AppointmentRepository;

/// Postgres-backed implementation of [`AppointmentRepository`].
pub struct PostgresAppointmentRepository {
    pool: PgPool,
}

impl PostgresAppointmentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Converts [`AppointmentStatus`] to its canonical database string.
///
/// Kept private to this module: no other layer should need to know the
/// wire format used in the `status` column.
fn status_to_str(status: AppointmentStatus) -> &'static str {
    match status {
        AppointmentStatus::Scheduled => "scheduled",
        AppointmentStatus::Completed => "completed",
        AppointmentStatus::Cancelled => "cancelled",
        AppointmentStatus::NoShow => "no_show",
    }
}

#[async_trait]
impl AppointmentRepository for PostgresAppointmentRepository {
    async fn insert(
        &self,
        patient_id: Uuid,
        doctor_id: Uuid,
        scheduled_for: DateTime<Utc>,
        notes: Option<&str>,
    ) -> Result<Appointment, CoreError> {
        // Foreign key violations (patient or doctor does not exist) surface
        // as SQLSTATE 23503. We map that to NotFound — from the caller's
        // perspective, "the patient you named does not exist" is not a
        // server error, it is a not-found.
        //
        // Unique violations (23505) would surface if we had a uniqueness
        // constraint on (doctor_id, scheduled_for). We do not have one yet;
        // conflict detection is handled by `has_conflict` in the service
        // layer. See the file header for the caveat about concurrency.
        let appointment = sqlx::query_as::<_, Appointment>(
            r#"
            INSERT INTO appointments (patient_id, doctor_id, scheduled_for, notes)
            VALUES ($1, $2, $3, $4)
            RETURNING
                id, patient_id, doctor_id, scheduled_for, status, notes,
                created_at, updated_at
            "#,
        )
            .bind(patient_id)
            .bind(doctor_id)
            .bind(scheduled_for)
            .bind(notes)
            .fetch_one(&self.pool)
            .await
            .map_err(|error| map_appointment_error(error, "insert appointment"))?;

        Ok(appointment)
    }

    async fn find_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<Appointment>, CoreError> {
        let appointment = sqlx::query_as::<_, Appointment>(
            r#"
            SELECT
                id, patient_id, doctor_id, scheduled_for, status, notes,
                created_at, updated_at
            FROM appointments
            WHERE id = $1
            "#,
        )
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| {
                CoreError::Internal(format!("find appointment by id: {error}"))
            })?;

        Ok(appointment)
    }

    async fn list_for_patient(
        &self,
        patient_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Appointment>, CoreError> {
        // Ordered by `scheduled_for DESC` because a patient's history is
        // most useful newest-first. For a "upcoming appointments" view you
        // would reverse the order and add `AND scheduled_for > NOW()`.
        let appointments = sqlx::query_as::<_, Appointment>(
            r#"
            SELECT
                id, patient_id, doctor_id, scheduled_for, status, notes,
                created_at, updated_at
            FROM appointments
            WHERE patient_id = $1
            ORDER BY scheduled_for DESC
            LIMIT $2
            "#,
        )
            .bind(patient_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| {
                CoreError::Internal(format!(
                    "list appointments for patient: {error}"
                ))
            })?;

        Ok(appointments)
    }

    async fn list_for_doctor(
        &self,
        doctor_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Appointment>, CoreError> {
        // `scheduled_for ASC` here, because a doctor's upcoming schedule is
        // most useful soonest-first.
        let appointments = sqlx::query_as::<_, Appointment>(
            r#"
            SELECT
                id, patient_id, doctor_id, scheduled_for, status, notes,
                created_at, updated_at
            FROM appointments
            WHERE doctor_id = $1
            ORDER BY scheduled_for ASC
            LIMIT $2
            "#,
        )
            .bind(doctor_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| {
                CoreError::Internal(format!(
                    "list appointments for doctor: {error}"
                ))
            })?;

        Ok(appointments)
    }

    async fn update_status(
        &self,
        id: Uuid,
        status: AppointmentStatus,
    ) -> Result<Appointment, CoreError> {
        // `status_to_str` is applied to the enum before binding, because
        // SQLx has no automatic mapping from our Rust enum to the TEXT
        // column. The reverse mapping (reading `AppointmentStatus` from
        // the row) is handled by the `#[sqlx(type_name = "TEXT")]`
        // attribute on the enum itself.
        let status_str = status_to_str(status);

        let appointment = sqlx::query_as::<_, Appointment>(
            r#"
            UPDATE appointments
            SET status = $2
            WHERE id = $1
            RETURNING
                id, patient_id, doctor_id, scheduled_for, status, notes,
                created_at, updated_at
            "#,
        )
            .bind(id)
            .bind(status_str)
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| {
                CoreError::Internal(format!(
                    "update appointment status: {error}"
                ))
            })?;

        appointment.ok_or_else(|| {
            CoreError::NotFound(format!("appointment {id} not found"))
        })
    }

    async fn has_conflict(
        &self,
        doctor_id: Uuid,
        scheduled_for: DateTime<Utc>,
        window_minutes: i64,
    ) -> Result<bool, CoreError> {
        // The query checks for any *active* appointment for the same doctor
        // whose `scheduled_for` is within `window_minutes` on either side of
        // the proposed time. Cancelled and no-show appointments are
        // ignored: they do not occupy the slot.
        //
        // We compare against `NOW()` only in the sense that expired
        // appointments are still rows — we do not filter by time because
        // the user may want to backfill a historical record. Business
        // policy on backdating belongs in the service.
        //
        // `EXISTS` is used instead of `COUNT(*)` because it can short
        // circuit on the first match. On a well-indexed column the
        // difference is negligible, but `EXISTS` also signals intent:
        // "is there at least one?".
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM appointments
                WHERE doctor_id = $1
                  AND status NOT IN ('cancelled', 'no_show')
                  AND scheduled_for BETWEEN
                      $2::timestamptz - ($3::int * INTERVAL '1 minute')
                      AND
                      $2::timestamptz + ($3::int * INTERVAL '1 minute')
            )
            "#,
        )
            .bind(doctor_id)
            .bind(scheduled_for)
            .bind(window_minutes)
            .fetch_one(&self.pool)
            .await
            .map_err(|error| {
                CoreError::Internal(format!("check appointment conflict: {error}"))
            })?;

        Ok(exists)
    }
}

/// Maps SQLx errors that occur during appointment writes to domain errors.
///
/// Foreign key violations (`23503`) mean a referenced patient or doctor
/// does not exist, which is a NotFound for the caller. Unique violations
/// (`23505`) would be a Conflict if we had a uniqueness constraint on the
/// schedule; we keep the mapping in place so the behaviour is correct the
/// moment we add one.
fn map_appointment_error(error: sqlx::Error, context: &str) -> CoreError {
    if let sqlx::Error::Database(db_error) = &error {
        match db_error.code().as_deref() {
            Some("23503") => {
                return CoreError::NotFound(format!(
                    "{context}: referenced entity does not exist"
                ));
            }
            Some("23505") => {
                return CoreError::Conflict(format!(
                    "{context}: duplicate appointment"
                ));
            }
            _ => {}
        }
    }
    CoreError::Internal(format!("{context}: {error}"))
}