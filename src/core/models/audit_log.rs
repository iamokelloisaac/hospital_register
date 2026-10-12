//! # Audit log model
//!
//! Every mutation of a clinical entity is recorded here. This is not optional
//! in a medical system: auditors and regulators need to reconstruct *who*
//! changed *what*, *when*, and *why*.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A single auditable event.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AuditLog {
    pub id: Uuid,
    pub actor_id: Option<Uuid>,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}