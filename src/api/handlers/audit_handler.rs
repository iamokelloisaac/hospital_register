//! # Audit handlers
//!
//! Read-only access to the audit log for administrators.

use axum::extract::{Query, State};
use axum::Extension;
use axum::Json;

use crate::api::dto::audit_dto::{AuditLogResponse, ListAuditQuery};
use crate::api::error::AppError;
use crate::app::state::AppState;
use crate::core::errors::CoreError;
use crate::services::auth_service::Claims;

/// `GET /api/v1/audit?entity_type=patient&entity_id=...`
///
/// Returns the audit history for a specific entity, newest first.
///
/// **Authorization:** admin only. Even a doctor should not see the full
/// audit trail of a patient they treat; audits are for compliance review,
/// not clinical use.
pub async fn list_audit_for_entity(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<ListAuditQuery>,
) -> Result<Json<Vec<AuditLogResponse>>, AppError> {
    if claims.role != "admin" {
        return Err(CoreError::Forbidden(
            "audit log access requires admin role".into(),
        )
            .into());
    }

    let limit = query.limit.clamp(1, 100);

    let entries = state
        .audit_service
        .history(&query.entity_type, query.entity_id, limit)
        .await?;

    Ok(Json(
        entries.into_iter().map(AuditLogResponse::from).collect(),
    ))
}