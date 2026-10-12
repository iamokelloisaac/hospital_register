//! Appointment request and response DTOs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::core::models::appointment::{Appointment, AppointmentStatus};

/// Request body for `POST /api/v1/appointments`.
#[derive(Debug, Deserialize, Validate)]
pub struct CreateAppointmentRequest {
    pub patient_id: Uuid,
    pub doctor_id: Uuid,

    /// ISO-8601 timestamp with timezone, e.g. `2025-06-01T10:00:00Z`.
    pub scheduled_for: DateTime<Utc>,

    #[validate(length(max = 2000))]
    pub notes: Option<String>,
}

/// Request body for `PATCH /api/v1/appointments/:id/status`.
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateAppointmentStatusRequest {
    pub status: AppointmentStatus,
}

/// Response body for an appointment.
#[derive(Debug, Serialize)]
pub struct AppointmentResponse {
    pub id: Uuid,
    pub patient_id: Uuid,
    pub doctor_id: Uuid,
    pub scheduled_for: DateTime<Utc>,
    pub status: AppointmentStatus,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Appointment> for AppointmentResponse {
    fn from(appointment: Appointment) -> Self {
        Self {
            id: appointment.id,
            patient_id: appointment.patient_id,
            doctor_id: appointment.doctor_id,
            scheduled_for: appointment.scheduled_for,
            status: appointment.status,
            notes: appointment.notes,
            created_at: appointment.created_at,
            updated_at: appointment.updated_at,
        }
    }
}