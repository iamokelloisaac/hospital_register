//! # Audit log repository (Postgres)
//!
//! Append-only log of every mutating action in the system.

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::core::errors::CoreError;
use crate::core::models::audit_log::AuditLog;
use crate::core::traits::AuditLogRepository;

pub struct PostgresAuditLogRepository {
    pool: PgPool,
}

impl PostgresAuditLogRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AuditLogRepository for PostgresAuditLogRepository {
    async fn append(
        &self,
        actor_id: Option<Uuid>,
        action: &str,
        entity_type: &str,
        entity_id: Uuid,
        metadata: Option<serde_json::Value>,
    ) -> Result<AuditLog, CoreError> {
        let row = sqlx::query_as::<_, AuditLog>(
            r#"
            INSERT INTO audit_logs (actor_id, action, entity_type, entity_id, metadata)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, actor_id, action, entity_type, entity_id, metadata, created_at
            "#,
        )
            .bind(actor_id)
            .bind(action)
            .bind(entity_type)
            .bind(entity_id)
            .bind(metadata)
            .fetch_one(&self.pool)
            .await
            .map_err(|error| {
                CoreError::Internal(format!("append audit log: {error}"))
            })?;

        Ok(row)
    }

    async fn list_for_entity(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        limit: i64,
    ) -> Result<Vec<AuditLog>, CoreError> {
        let rows = sqlx::query_as::<_, AuditLog>(
            r#"
            SELECT id, actor_id, action, entity_type, entity_id, metadata, created_at
            FROM audit_logs
            WHERE entity_type = $1 AND entity_id = $2
            ORDER BY created_at DESC
            LIMIT $3
            "#,
        )
            .bind(entity_type)
            .bind(entity_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| {
                CoreError::Internal(format!("list audit logs: {error}"))
            })?;

        Ok(rows)
    }
}