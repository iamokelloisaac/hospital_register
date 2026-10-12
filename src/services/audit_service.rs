//! # Audit service
//!
//! Thin wrapper around [`AuditLogRepository`] that adds domain semantics
//! (e.g., "the actor who did this", "what action name"). Services call
//! this rather than touching the repository directly so that the audit
//! vocabulary stays consistent.

use std::sync::Arc;
use uuid::Uuid;

use crate::core::errors::CoreError;
use crate::core::models::audit_log::AuditLog;
use crate::core::traits::AuditLogRepository;

pub struct AuditService {
    repository: Arc<dyn AuditLogRepository>,
}

impl AuditService {
    pub fn new(repository: Arc<dyn AuditLogRepository>) -> Self {
        Self { repository }
    }

    /// Records an action against an entity.
    pub async fn record(
        &self,
        actor_id: Option<Uuid>,
        action: &str,
        entity_type: &str,
        entity_id: Uuid,
        metadata: Option<serde_json::Value>,
    ) -> Result<AuditLog, CoreError> {
        self.repository
            .append(actor_id, action, entity_type, entity_id, metadata)
            .await
    }

    /// Returns the recent history of an entity.
    pub async fn history(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        limit: i64,
    ) -> Result<Vec<AuditLog>, CoreError> {
        self.repository
            .list_for_entity(entity_type, entity_id, limit)
            .await
    }
}