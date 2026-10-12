//! # Patient service
//!
//! Business logic for patient registration, retrieval, update, and
//! soft-delete. Every mutation writes an audit log entry.

use std::sync::Arc;
use uuid::Uuid;

use crate::core::errors::CoreError;
use crate::core::models::patient::{BloodType, Patient};
use crate::core::traits::PatientRepository;
use crate::services::audit_service::AuditService;

pub struct PatientService {
    patient_repository: Arc<dyn PatientRepository>,
    audit_service: Arc<AuditService>,
}

impl PatientService {
    pub fn new(
        patient_repository: Arc<dyn PatientRepository>,
        audit_service: Arc<AuditService>,
    ) -> Self {
        Self {
            patient_repository,
            audit_service,
        }
    }

    /// Registers a new patient and records an audit entry.
    ///
    /// # Validation
    ///
    /// Business-level validation lives here, not in the HTTP layer. The
    /// DTO layer enforces shape (e.g., `nin` is non-empty); the service
    /// enforces domain rules (e.g., a patient cannot be born in the future).
    pub async fn register(
        &self,
        actor_id: Option<Uuid>,
        nin: &str,
        first_name: &str,
        last_name: &str,
        date_of_birth: chrono::NaiveDate,
        blood_type: Option<BloodType>,
    ) -> Result<Patient, CoreError> {
        // Business rule: date of birth must not be in the future.
        let today = chrono::Utc::now().date_naive();
        if date_of_birth > today {
            return Err(CoreError::Validation(
                "date of birth cannot be in the future".into(),
            ));
        }

        // Business rule: names must not be blank after trimming.
        if first_name.trim().is_empty() || last_name.trim().is_empty() {
            return Err(CoreError::Validation(
                "first name and last name must not be blank".into(),
            ));
        }

        let patient = self
            .patient_repository
            .insert(
                nin,
                first_name.trim(),
                last_name.trim(),
                date_of_birth,
                blood_type,
            )
            .await?;

        // Audit the creation. Metadata carries the NIN so auditors can
        // cross-reference without joining to the (possibly soft-deleted)
        // patient row.
        self.audit_service
            .record(
                actor_id,
                "patient.created",
                "patient",
                patient.id,
                Some(serde_json::json!({ "nin": patient.nin })),
            )
            .await?;

        Ok(patient)
    }

    /// Retrieves a patient by ID.
    pub async fn get(&self, id: Uuid) -> Result<Patient, CoreError> {
        self.patient_repository
            .find_by_id(id)
            .await?
            .ok_or_else(|| {
                CoreError::NotFound(format!("patient {id} not found"))
            })
    }

    /// Lists active patients.
    pub async fn list(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Patient>, CoreError> {
        // Clamp the page size to protect the database from unbounded scans.
        let limit = limit.clamp(1, 100);
        let offset = offset.max(0);
        self.patient_repository.list(limit, offset).await
    }

    /// Updates mutable fields on a patient.
    pub async fn update(
        &self,
        actor_id: Option<Uuid>,
        id: Uuid,
        first_name: Option<&str>,
        last_name: Option<&str>,
        blood_type: Option<BloodType>,
    ) -> Result<Patient, CoreError> {
        let patient = self
            .patient_repository
            .update(id, first_name, last_name, blood_type)
            .await?;

        self.audit_service
            .record(
                actor_id,
                "patient.updated",
                "patient",
                patient.id,
                None,
            )
            .await?;

        Ok(patient)
    }

    /// Soft-deletes a patient.
    pub async fn delete(
        &self,
        actor_id: Option<Uuid>,
        id: Uuid,
    ) -> Result<(), CoreError> {
        self.patient_repository.soft_delete(id).await?;

        self.audit_service
            .record(
                actor_id,
                "patient.deleted",
                "patient",
                id,
                None,
            )
            .await?;

        Ok(())
    }
}