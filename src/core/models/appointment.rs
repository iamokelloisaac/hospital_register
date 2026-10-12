//! # Appointment domain model
//!
//! An appointment links a patient to a doctor at a specific time. It is the
//! primary scheduling primitive of the system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A scheduled interaction between a patient and a doctor.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Appointment {
    pub id: Uuid,
    pub patient_id: Uuid,
    pub doctor_id: Uuid,
    pub scheduled_for: DateTime<Utc>,
    pub status: AppointmentStatus,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// The lifecycle states of an appointment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[serde(rename_all = "snake_case")]
pub enum AppointmentStatus {
    Scheduled,
    Completed,
    Cancelled,
    NoShow,
}