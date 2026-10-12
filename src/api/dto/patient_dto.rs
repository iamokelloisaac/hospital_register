//! Patient request and response DTOs.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::core::models::patient::{BloodType, Patient};

/// Request body for `POST /api/v1/patients`.
#[derive(Debug, Deserialize, Validate)]
pub struct CreatePatientRequest {
    /// National Identification Number.
    #[validate(length(min = 4, max = 32))]
    pub nin: String,

    #[validate(length(min = 1, max = 100))]
    pub first_name: String,

    #[validate(length(min = 1, max = 100))]
    pub last_name: String,

    pub date_of_birth: NaiveDate,

    pub blood_type: Option<BloodType>,
}

/// Request body for `PATCH /api/v1/patients/:id`.
///
/// All fields are optional: only provided fields are updated.
#[derive(Debug, Deserialize, Validate)]
pub struct UpdatePatientRequest {
    #[validate(length(min = 1, max = 100))]
    pub first_name: Option<String>,

    #[validate(length(min = 1, max = 100))]
    pub last_name: Option<String>,

    pub blood_type: Option<BloodType>,
}

/// Response body representing a patient.
#[derive(Debug, Serialize)]
pub struct PatientResponse {
    pub id: Uuid,
    pub nin: String,
    pub first_name: String,
    pub last_name: String,
    pub date_of_birth: NaiveDate,
    pub blood_type: Option<BloodType>,
    pub created_at: DateTime<Utc>,
}

impl From<Patient> for PatientResponse {
    fn from(patient: Patient) -> Self {
        Self {
            id: patient.id,
            nin: patient.nin,
            first_name: patient.first_name,
            last_name: patient.last_name,
            date_of_birth: patient.date_of_birth,
            blood_type: patient.blood_type,
            created_at: patient.created_at,
        }
    }
}