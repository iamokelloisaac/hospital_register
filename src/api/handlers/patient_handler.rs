//! Patient handlers.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Extension;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::api::dto::patient_dto::{
    CreatePatientRequest, PatientResponse, UpdatePatientRequest,
};
use crate::api::error::AppError;
use crate::api::extractors::validated_json::ValidatedJson;
use crate::app::state::AppState;
use crate::services::auth_service::Claims;

/// Query parameters for `GET /api/v1/patients`.
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

pub(crate) fn default_limit() -> i64 {
    20
}

/// `POST /api/v1/patients`
pub async fn create_patient(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    ValidatedJson(body): ValidatedJson<CreatePatientRequest>,
) -> Result<impl IntoResponse, AppError> {
    let patient = state
        .patient_service
        .register(
            Some(claims.sub),
            &body.nin,
            &body.first_name,
            &body.last_name,
            body.date_of_birth,
            body.blood_type,
        )
        .await?;

    Ok((StatusCode::CREATED, Json(PatientResponse::from(patient))))
}

/// `GET /api/v1/patients/:id`
pub async fn get_patient(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<PatientResponse>, AppError> {
    let patient = state.patient_service.get(id).await?;
    Ok(Json(PatientResponse::from(patient)))
}

/// `GET /api/v1/patients`
pub async fn list_patients(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<PatientResponse>>, AppError> {
    let patients = state
        .patient_service
        .list(query.limit, query.offset)
        .await?;

    Ok(Json(
        patients.into_iter().map(PatientResponse::from).collect(),
    ))
}

/// `PATCH /api/v1/patients/:id`
pub async fn update_patient(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<UpdatePatientRequest>,
) -> Result<Json<PatientResponse>, AppError> {
    let patient = state
        .patient_service
        .update(
            Some(claims.sub),
            id,
            body.first_name.as_deref(),
            body.last_name.as_deref(),
            body.blood_type,
        )
        .await?;

    Ok(Json(PatientResponse::from(patient)))
}

/// `DELETE /api/v1/patients/:id`
pub async fn delete_patient(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    state.patient_service.delete(Some(claims.sub), id).await?;
    Ok(StatusCode::NO_CONTENT)
}