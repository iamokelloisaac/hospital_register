//! # Appointment service
//!
//! Business logic for booking, retrieving, and updating appointments.
//!
//! ## Conflict detection
//!
//! Before inserting, we ask the repository whether the doctor already has
//! an active appointment within a small window of the proposed time. The
//! window is configurable at the call site and currently 15 minutes.
//!
//! This is a *check-then-insert*, not a lock. Under concurrent load two
//! requests can both pass the check before either insert commits, and both
//! appointments will be created. A future migration will add a Postgres
//! exclusion constraint on `(doctor_id, tstzrange(scheduled_for, ...))` to
//! make this atomic. Until then, the service is honest about the limit.

use std::sync::Arc;
use uuid::Uuid;

use crate::core::errors::CoreError;
use crate::core::models::appointment::{Appointment, AppointmentStatus};
use crate::core::traits::AppointmentRepository;
use crate::services::audit_service::AuditService;

/// Minutes on either side of the proposed time that count as a conflict.
///
/// A doctor cannot be booked twice within this window. For example, with
/// a value of 15, an appointment at 10:00 conflicts with any active
/// appointment from 09:45 to 10:15.
const CONFLICT_WINDOW_MINUTES: i64 = 15;

pub struct AppointmentService {
    appointment_repository: Arc<dyn AppointmentRepository>,
    audit_service: Arc<AuditService>,
}

impl AppointmentService {
    pub fn new(
        appointment_repository: Arc<dyn AppointmentRepository>,
        audit_service: Arc<AuditService>,
    ) -> Self {
        Self {
            appointment_repository,
            audit_service,
        }
    }

    /// Books a new appointment.
    ///
    /// Rejects the booking if the doctor is already committed within
    /// [`CONFLICT_WINDOW_MINUTES`] of the requested time.
    pub async fn book(
        &self,
        actor_id: Option<Uuid>,
        patient_id: Uuid,
        doctor_id: Uuid,
        scheduled_for: chrono::DateTime<chrono::Utc>,
        notes: Option<&str>,
    ) -> Result<Appointment, CoreError> {
        // Business rule: the appointment must be in the future. Booking a
        // slot in the past is almost always a mistake and would confuse
        // scheduling. If backdating is ever needed, add an explicit
        // parameter rather than relaxing this check.
        if scheduled_for <= chrono::Utc::now() {
            return Err(CoreError::Validation(
                "appointment must be scheduled in the future".into(),
            ));
        }

        // Conflict check: refuse if the doctor is already booked nearby.
        let conflict = self
            .appointment_repository
            .has_conflict(
                doctor_id,
                scheduled_for,
                CONFLICT_WINDOW_MINUTES,
            )
            .await?;

        if conflict {
            return Err(CoreError::Conflict(format!(
                "doctor {doctor_id} already has an appointment near {scheduled_for}"
            )));
        }

        let appointment = self
            .appointment_repository
            .insert(patient_id, doctor_id, scheduled_for, notes)
            .await?;

        self.audit_service
            .record(
                actor_id,
                "appointment.created",
                "appointment",
                appointment.id,
                Some(serde_json::json!({
                    "patient_id": patient_id,
                    "doctor_id": doctor_id,
                    "scheduled_for": scheduled_for,
                })),
            )
            .await?;

        Ok(appointment)
    }

    /// Retrieves an appointment by ID.
    pub async fn get(&self, id: Uuid) -> Result<Appointment, CoreError> {
        self.appointment_repository
            .find_by_id(id)
            .await?
            .ok_or_else(|| {
                CoreError::NotFound(format!(
                    "appointment {id} not found"
                ))
            })
    }

    /// Lists appointments for a patient.
    pub async fn list_for_patient(
        &self,
        patient_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Appointment>, CoreError> {
        let limit = limit.clamp(1, 100);
        self.appointment_repository
            .list_for_patient(patient_id, limit)
            .await
    }

    /// Lists upcoming appointments for a doctor.
    pub async fn list_for_doctor(
        &self,
        doctor_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Appointment>, CoreError> {
        let limit = limit.clamp(1, 100);
        self.appointment_repository
            .list_for_doctor(doctor_id, limit)
            .await
    }

    /// Updates the status of an appointment.
    ///
    /// # State transition rules
    ///
    /// A terminal status (completed, cancelled, no_show) cannot be changed.
    /// A scheduled appointment can move to any terminal status. Allowing
    /// these transitions is the service's responsibility, not the
    /// repository's.
    pub async fn update_status(
        &self,
        actor_id: Option<Uuid>,
        id: Uuid,
        new_status: AppointmentStatus,
    ) -> Result<Appointment, CoreError> {
        let current = self.get(id).await?;

        // Terminal states are immutable. Once an appointment is completed,
        // cancelled, or marked no-show, it does not change. This protects
        // the audit trail.
        let current_is_terminal = matches!(
            current.status,
            AppointmentStatus::Completed
                | AppointmentStatus::Cancelled
                | AppointmentStatus::NoShow
        );

        if current_is_terminal {
            return Err(CoreError::Conflict(format!(
                "cannot change status from {:?} to {:?}",
                current.status, new_status
            )));
        }

        let updated = self
            .appointment_repository
            .update_status(id, new_status)
            .await?;

        self.audit_service
            .record(
                actor_id,
                "appointment.status_changed",
                "appointment",
                id,
                Some(serde_json::json!({
                    "from": format!("{:?}", current.status),
                    "to": format!("{:?}", new_status),
                })),
            )
            .await?;

        Ok(updated)
    }
}