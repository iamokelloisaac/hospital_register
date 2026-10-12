//! Doctor request and response DTOs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::core::models::doctor::Doctor;

/// Request body for `POST /api/v1/doctors`.
///
/// Note that `password` is a plaintext string in the request. It is
/// hashed in the service layer before it touches any storage. The
/// `#[validate(length(min = 8))]` bound is the first line of defense;
/// the service can add stricter rules (e.g., character classes) later.
#[derive(Debug, Deserialize, Validate)]
pub struct CreateDoctorRequest {
    #[validate(email)]
    pub email: String,

    #[validate(length(min = 8, max = 128))]
    pub password: String,

    #[validate(length(min = 1, max = 200))]
    pub full_name: String,

    #[validate(length(min = 1, max = 100))]
    pub specialty: String,

    /// One of `doctor`, `admin`, `receptionist`. Validated in the service.
    pub role: String,
}

/// Request body for `PATCH /api/v1/doctors/:id`.
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateDoctorRequest {
    #[validate(length(min = 1, max = 200))]
    pub full_name: Option<String>,

    #[validate(length(min = 1, max = 100))]
    pub specialty: Option<String>,
}

/// Response body for a doctor.
///
/// Note the deliberate absence of `password_hash`. That field lives only
/// on [`DoctorWithHash`](crate::core::traits::DoctorWithHash), which is
/// never serialized. It is therefore impossible for a response to leak
/// the hash, even by mistake.
#[derive(Debug, Serialize)]
pub struct DoctorResponse {
    pub id: Uuid,
    pub email: String,
    pub full_name: String,
    pub specialty: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

impl From<Doctor> for DoctorResponse {
    fn from(doctor: Doctor) -> Self {
        Self {
            id: doctor.id,
            email: doctor.email,
            full_name: doctor.full_name,
            specialty: doctor.specialty,
            role: doctor.role,
            created_at: doctor.created_at,
        }
    }
}