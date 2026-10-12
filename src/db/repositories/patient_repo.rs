//! # Patient repository (Postgres)
//!
//! Implements [`PatientRepository`] using SQLx.
//!
//! ## Error mapping
//!
//! SQLx errors are translated to [`CoreError`] at this boundary. Nothing
//! above this module ever sees a `sqlx::Error`.
//!
//! - `sqlx::Error::Database` with SQLSTATE `23505` (unique_violation)
//!   → [`CoreError::Conflict`]
//! - `sqlx::Error::RowNotFound` → [`CoreError::NotFound`]
//! - Anything else → [`CoreError::Internal`]
//!
//! ## Why runtime queries and not `sqlx::query!`?
//!
//! The `query!` macro checks queries against a live database at compile
//! time — a powerful feature, but it requires either a running Postgres at
//! build time or a committed `sqlx-data.json` cache. For a tutorial codebase
//! that must compile in any environment, we use the runtime variants
//! ([`sqlx::query_as`], [`sqlx::query`]). In production, prefer `query!`.

use async_trait::async_trait;
use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;

use crate::core::errors::CoreError;
use crate::core::models::patient::{BloodType, Patient};
use crate::core::traits::PatientRepository;

/// Postgres-backed implementation of [`PatientRepository`].
pub struct PostgresPatientRepository {
    pool: PgPool,
}

impl PostgresPatientRepository {
    /// Creates a new repository backed by the given pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Detects unique-constraint violations from SQLx.
///
/// Postgres reports a unique violation with SQLSTATE `23505`. SQLx surfaces
/// this through `sqlx::Error::Database`, from which we can read the code.
fn is_unique_violation(error: &sqlx::Error) -> bool {
    match error {
        sqlx::Error::Database(db_error) => {
            db_error.code().as_deref() == Some("23505")
        }
        _ => false,
    }
}

/// Maps a `sqlx::Error` into a `CoreError` for the "insert" path.
fn map_insert_error(error: sqlx::Error, context: &str) -> CoreError {
    if is_unique_violation(&error) {
        return CoreError::Conflict(format!("{context}: already exists"));
    }
    CoreError::Internal(format!("{context}: {error}"))
}

#[async_trait]
impl PatientRepository for PostgresPatientRepository {
    async fn insert(
        &self,
        nin: &str,
        first_name: &str,
        last_name: &str,
        date_of_birth: NaiveDate,
        blood_type: Option<BloodType>,
    ) -> Result<Patient, CoreError> {
        // We pass `blood_type` as a string because SQLx maps our enum's
        // `#[sqlx(type_name = "TEXT")]` to TEXT, and we have to serialize
        // it ourselves here. A helper on the enum would also work.
        let blood_type_str: Option<String> = blood_type.map(|bt| match bt {
            BloodType::APositive => "A+".to_string(),
            BloodType::ANegative => "A-".to_string(),
            BloodType::BPositive => "B+".to_string(),
            BloodType::BNegative => "B-".to_string(),
            BloodType::ABPositive => "AB+".to_string(),
            BloodType::ABNegative => "AB-".to_string(),
            BloodType::OPositive => "O+".to_string(),
            BloodType::ONegative => "O-".to_string(),
        });

        let row = sqlx::query_as::<_, Patient>(
            r#"
            INSERT INTO patients (nin, first_name, last_name, date_of_birth, blood_type)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING
                id, nin, first_name, last_name, date_of_birth,
                blood_type, created_at, updated_at, deleted_at
            "#,
        )
            .bind(nin)
            .bind(first_name)
            .bind(last_name)
            .bind(date_of_birth)
            .bind(blood_type_str)
            .fetch_one(&self.pool)
            .await
            .map_err(|error| map_insert_error(error, "patient"))?;

        Ok(row)
    }

    async fn find_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<Patient>, CoreError> {
        let row = sqlx::query_as::<_, Patient>(
            r#"
            SELECT
                id, nin, first_name, last_name, date_of_birth,
                blood_type, created_at, updated_at, deleted_at
            FROM patients
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| {
                CoreError::Internal(format!("find patient by id: {error}"))
            })?;

        Ok(row)
    }

    async fn list(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Patient>, CoreError> {
        let rows = sqlx::query_as::<_, Patient>(
            r#"
            SELECT
                id, nin, first_name, last_name, date_of_birth,
                blood_type, created_at, updated_at, deleted_at
            FROM patients
            WHERE deleted_at IS NULL
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| {
                CoreError::Internal(format!("list patients: {error}"))
            })?;

        Ok(rows)
    }

    async fn update(
        &self,
        id: Uuid,
        first_name: Option<&str>,
        last_name: Option<&str>,
        blood_type: Option<BloodType>,
    ) -> Result<Patient, CoreError> {
        // COALESCE returns the first non-NULL argument. This lets one SQL
        // statement handle partial updates: pass NULL to leave a field alone.
        let blood_type_str: Option<String> = blood_type.map(|bt| match bt {
            BloodType::APositive => "A+".to_string(),
            BloodType::ANegative => "A-".to_string(),
            BloodType::BPositive => "B+".to_string(),
            BloodType::BNegative => "B-".to_string(),
            BloodType::ABPositive => "AB+".to_string(),
            BloodType::ABNegative => "AB-".to_string(),
            BloodType::OPositive => "O+".to_string(),
            BloodType::ONegative => "O-".to_string(),
        });

        let row = sqlx::query_as::<_, Patient>(
            r#"
            UPDATE patients
            SET
                first_name = COALESCE($2, first_name),
                last_name  = COALESCE($3, last_name),
                blood_type = COALESCE($4, blood_type)
            WHERE id = $1 AND deleted_at IS NULL
            RETURNING
                id, nin, first_name, last_name, date_of_birth,
                blood_type, created_at, updated_at, deleted_at
            "#,
        )
            .bind(id)
            .bind(first_name)
            .bind(last_name)
            .bind(blood_type_str)
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| {
                CoreError::Internal(format!("update patient: {error}"))
            })?;

        row.ok_or_else(|| {
            CoreError::NotFound(format!("patient {id} not found"))
        })
    }

    async fn soft_delete(&self, id: Uuid) -> Result<(), CoreError> {
        let result = sqlx::query(
            r#"
            UPDATE patients
            SET deleted_at = NOW()
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|error| {
                CoreError::Internal(format!("soft-delete patient: {error}"))
            })?;

        if result.rows_affected() == 0 {
            return Err(CoreError::NotFound(format!(
                "patient {id} not found"
            )));
        }

        Ok(())
    }
}