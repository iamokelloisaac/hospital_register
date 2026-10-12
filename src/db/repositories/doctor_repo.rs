//! # Doctor repository (Postgres)
//!
//! Implements [`DoctorRepository`] using SQLx.
//!
//! ## Error mapping
//!
//! SQLx errors are translated to [`CoreError`] at this boundary, exactly as
//! in `patient_repo`:
//!
//! - SQLSTATE `23505` (unique_violation) → [`CoreError::Conflict`]
//! - `sqlx::Error::RowNotFound` → [`CoreError::NotFound`]
//! - Anything else → [`CoreError::Internal`]
//!
//! ## Why `find_by_email_with_hash` is separate
//!
//! The auth flow needs the password hash; the CRUD flow must not see it.
//! We enforce this at the type level: `find_by_id` returns [`Doctor`] (no
//! hash field), and only `find_by_email_with_hash` returns
//! [`DoctorWithHash`]. It is impossible for a response DTO to accidentally
//! serialize the hash because the source struct does not carry it.

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::core::errors::CoreError;
use crate::core::models::doctor::Doctor;
use crate::core::traits::{DoctorRepository, DoctorWithHash};

/// Postgres-backed implementation of [`DoctorRepository`].
pub struct PostgresDoctorRepository {
    pool: PgPool,
}

impl PostgresDoctorRepository {
    /// Creates a new repository backed by the given pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Detects unique-constraint violations from SQLx.
///
/// Postgres reports a unique violation with SQLSTATE `23505`.
fn is_unique_violation(error: &sqlx::Error) -> bool {
    match error {
        sqlx::Error::Database(db_error) => {
            db_error.code().as_deref() == Some("23505")
        }
        _ => false,
    }
}

/// Maps a `sqlx::Error` into a `CoreError` for insert-like operations.
fn map_insert_error(error: sqlx::Error, context: &str) -> CoreError {
    if is_unique_violation(&error) {
        return CoreError::Conflict(format!("{context}: already exists"));
    }
    CoreError::Internal(format!("{context}: {error}"))
}

#[async_trait]
impl DoctorRepository for PostgresDoctorRepository {
    async fn insert(
        &self,
        email: &str,
        password_hash: &str,
        full_name: &str,
        specialty: &str,
        role: &str,
    ) -> Result<Doctor, CoreError> {
        // The `RETURNING` clause returns the row the database just wrote,
        // including the server-generated `id`, `created_at`, and
        // `updated_at`. This avoids a second round trip.
        //
        // Note: we never select `password_hash` here. That column is only
        // read by `find_by_email_with_hash`.
        let doctor = sqlx::query_as::<_, Doctor>(
            r#"
            INSERT INTO doctors (email, password_hash, full_name, specialty, role)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING
                id, email, full_name, specialty, role,
                created_at, updated_at, deleted_at
            "#,
        )
            .bind(email)
            .bind(password_hash)
            .bind(full_name)
            .bind(specialty)
            .bind(role)
            .fetch_one(&self.pool)
            .await
            .map_err(|error| map_insert_error(error, "doctor"))?;

        Ok(doctor)
    }

    async fn find_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<Doctor>, CoreError> {
        // `fetch_optional` yields `Ok(None)` on no match, which is the
        // semantics we want: "not found" is not an error at this layer.
        // The service decides whether to translate it into an error.
        let doctor = sqlx::query_as::<_, Doctor>(
            r#"
            SELECT
                id, email, full_name, specialty, role,
                created_at, updated_at, deleted_at
            FROM doctors
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| {
                CoreError::Internal(format!("find doctor by id: {error}"))
            })?;

        Ok(doctor)
    }

    async fn find_by_email_with_hash(
        &self,
        email: &str,
    ) -> Result<Option<DoctorWithHash>, CoreError> {
        // This is the only query in the codebase that reads `password_hash`.
        // Keeping it isolated makes audits straightforward: grep for
        // `password_hash` and you find every code path that touches it.
        let row = sqlx::query_as::<_, DoctorWithHash>(
            r#"
            SELECT id, email, full_name, role, password_hash
            FROM doctors
            WHERE email = $1 AND deleted_at IS NULL
            "#,
        )
            .bind(email)
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| {
                CoreError::Internal(format!("find doctor by email: {error}"))
            })?;

        Ok(row)
    }

    async fn list(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Doctor>, CoreError> {
        // `ORDER BY created_at DESC` gives newest first, which is the
        // intuitive default for an admin listing. Add other sort keys
        // later if needed, but keep the ordering deterministic.
        let doctors = sqlx::query_as::<_, Doctor>(
            r#"
            SELECT
                id, email, full_name, specialty, role,
                created_at, updated_at, deleted_at
            FROM doctors
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
                CoreError::Internal(format!("list doctors: {error}"))
            })?;

        Ok(doctors)
    }

    async fn update(
        &self,
        id: Uuid,
        full_name: Option<&str>,
        specialty: Option<&str>,
    ) -> Result<Doctor, CoreError> {
        // `COALESCE($n, column)` returns the first non-NULL argument.
        // Passing `NULL` in the bind position therefore leaves the existing
        // value untouched. This lets one statement handle partial updates
        // without dynamic SQL.
        //
        // We deliberately do not allow updating `email`, `role`, or
        // `password_hash` here. Those are changed via dedicated flows
        // (profile email change, role change by admin, password reset) that
        // have their own authorization and audit requirements.
        let doctor = sqlx::query_as::<_, Doctor>(
            r#"
            UPDATE doctors
            SET
                full_name = COALESCE($2, full_name),
                specialty = COALESCE($3, specialty)
            WHERE id = $1 AND deleted_at IS NULL
            RETURNING
                id, email, full_name, specialty, role,
                created_at, updated_at, deleted_at
            "#,
        )
            .bind(id)
            .bind(full_name)
            .bind(specialty)
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| {
                CoreError::Internal(format!("update doctor: {error}"))
            })?;

        // `fetch_optional` because a missing row is a not-found, not a
        // database error. Translate it to a domain error here, at the
        // boundary.
        doctor.ok_or_else(|| {
            CoreError::NotFound(format!("doctor {id} not found"))
        })
    }

    async fn soft_delete(&self, id: Uuid) -> Result<(), CoreError> {
        // Soft delete: set `deleted_at`. The partial indexes we created
        // in the migration (`WHERE deleted_at IS NULL`) exclude this row
        // from future lookups without any application-level filter needed
        // on top of the WHERE clause already present in every query.
        //
        // Rows-affected tells us whether the ID existed and was active.
        // Zero means either it never existed or it was already deleted.
        // Either way, from the caller's perspective it is "not found".
        let result = sqlx::query(
            r#"
            UPDATE doctors
            SET deleted_at = NOW()
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|error| {
                CoreError::Internal(format!("soft-delete doctor: {error}"))
            })?;

        if result.rows_affected() == 0 {
            return Err(CoreError::NotFound(format!(
                "doctor {id} not found"
            )));
        }

        Ok(())
    }
}