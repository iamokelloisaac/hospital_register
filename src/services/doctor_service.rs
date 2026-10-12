//! # Doctor service
//!
//! Business logic for doctor registration, profile management, and
//! soft-delete. Password hashing happens here, in the service layer, so
//! no repository or handler ever sees a plaintext password.
//!
//! ## Authorization note
//!
//! Creating a doctor is an administrative action. For now we do not
//! enforce this because we have no role-based middleware yet. Once
//! [`crate::api::middleware`] grows a `require_role` layer, the handler
//! will gate this route. The service deliberately does *not* check roles:
//! authorization is a concern of the API boundary, not of domain logic.

use std::sync::Arc;
use uuid::Uuid;

use crate::core::errors::CoreError;
use crate::core::models::doctor::Doctor;
use crate::core::traits::DoctorRepository;
use crate::services::audit_service::AuditService;
use crate::shared::crypto::hash_password;

pub struct DoctorService {
    doctor_repository: Arc<dyn DoctorRepository>,
    audit_service: Arc<AuditService>,
}

impl DoctorService {
    pub fn new(
        doctor_repository: Arc<dyn DoctorRepository>,
        audit_service: Arc<AuditService>,
    ) -> Self {
        Self {
            doctor_repository,
            audit_service,
        }
    }

    /// Registers a new doctor.
    ///
    /// The plaintext password is hashed with Argon2id before it leaves this
    /// method. The hash is the only form that reaches the repository.
    pub async fn register(
        &self,
        actor_id: Option<Uuid>,
        email: &str,
        password: &str,
        full_name: &str,
        specialty: &str,
        role: &str,
    ) -> Result<Doctor, CoreError> {
        // Business rule: role must be one of the three known values. We
        // could rely on the database CHECK constraint, but catching this
        // here produces a cleaner error message and avoids a round trip.
        if !matches!(role, "doctor" | "admin" | "receptionist") {
            return Err(CoreError::Validation(format!(
                "invalid role '{role}': must be doctor, admin, or receptionist"
            )));
        }

        if full_name.trim().is_empty() {
            return Err(CoreError::Validation(
                "full name must not be blank".into(),
            ));
        }

        if specialty.trim().is_empty() {
            return Err(CoreError::Validation(
                "specialty must not be blank".into(),
            ));
        }

        // Hash the password before persisting. This is the only place in
        // the codebase that transitions from plaintext to hash.
        let password_hash = hash_password(password)?;

        let doctor = self
            .doctor_repository
            .insert(
                email,
                &password_hash,
                full_name.trim(),
                specialty.trim(),
                role,
            )
            .await?;

        self.audit_service
            .record(
                actor_id,
                "doctor.created",
                "doctor",
                doctor.id,
                Some(serde_json::json!({
                    "email": doctor.email,
                    "role": doctor.role,
                })),
            )
            .await?;

        Ok(doctor)
    }

    /// Retrieves a doctor by ID.
    pub async fn get(&self, id: Uuid) -> Result<Doctor, CoreError> {
        self.doctor_repository
            .find_by_id(id)
            .await?
            .ok_or_else(|| {
                CoreError::NotFound(format!("doctor {id} not found"))
            })
    }

    /// Lists active doctors with pagination.
    pub async fn list(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Doctor>, CoreError> {
        // Clamp the page size, same as PatientService.
        let limit = limit.clamp(1, 100);
        let offset = offset.max(0);
        self.doctor_repository.list(limit, offset).await
    }

    /// Updates a doctor's profile.
    pub async fn update(
        &self,
        actor_id: Option<Uuid>,
        id: Uuid,
        full_name: Option<&str>,
        specialty: Option<&str>,
    ) -> Result<Doctor, CoreError> {
        // Trim and validate any provided string fields before passing
        // them down. `None` passes through unchanged (no update).
        let full_name = full_name.map(str::trim).filter(|s| !s.is_empty());
        let specialty = specialty.map(str::trim).filter(|s| !s.is_empty());

        let doctor = self
            .doctor_repository
            .update(id, full_name, specialty)
            .await?;

        self.audit_service
            .record(
                actor_id,
                "doctor.updated",
                "doctor",
                doctor.id,
                None,
            )
            .await?;

        Ok(doctor)
    }

    /// Soft-deletes a doctor.
    pub async fn delete(
        &self,
        actor_id: Option<Uuid>,
        id: Uuid,
    ) -> Result<(), CoreError> {
        self.doctor_repository.soft_delete(id).await?;

        self.audit_service
            .record(actor_id, "doctor.deleted", "doctor", id, None)
            .await?;

        Ok(())
    }
}