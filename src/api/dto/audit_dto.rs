//! Audit log DTOs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::models::audit_log::AuditLog;

/// Query parameters for `GET /api/v1/audit`.
///
/// Both `entity_type` and `entity_id` are required. There is no "list all
/// audit logs" query, because the table grows without bound and an
/// unbounded read is both expensive and a privacy risk.
#[derive(Debug, Deserialize)]
pub struct ListAuditQuery {
    pub entity_type: String,
    pub entity_id: Uuid,
    #[serde(default = "crate::api::handlers::patient_handler::default_limit")]
    pub limit: i64,
}

/// Response body for a single audit entry.
#[derive(Debug, Serialize)]
pub struct AuditLogResponse {
    pub id: Uuid,
    pub actor_id: Option<Uuid>,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl From<AuditLog> for AuditLogResponse {
    fn from(entry: AuditLog) -> Self {
        Self {
            id: entry.id,
            actor_id: entry.actor_id,
            action: entry.action,
            entity_type: entry.entity_type,
            entity_id: entry.entity_id,
            metadata: entry.metadata,
            created_at: entry.created_at,
        }
    }
}