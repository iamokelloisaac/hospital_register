//! Doctor handlers.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Extension;
use axum::Json;
use uuid::Uuid;

use crate::api::dto::doctor_dto::{
    CreateDoctorRequest, DoctorResponse, UpdateDoctorRequest,
};
use crate::api::error::AppError;
use crate::api::extractors::validated_json::ValidatedJson;
use crate::api::handlers::patient_handler::ListQuery;
use crate::app::state::AppState;
use crate::core::errors::CoreError;
use crate::services::auth_service::Claims;

/// `POST /api/v1/doctors`
///
/// **Authorization:** admin only. Enforced inline for now; will move to
/// middleware when `require_role` lands.
pub async fn create_doctor(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    ValidatedJson(body): ValidatedJson<CreateDoctorRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_role(&claims, "admin")?;

    let doctor = state
        .doctor_service
        .register(
            Some(claims.sub),
            &body.email,
            &body.password,
            &body.full_name,
            &body.specialty,
            &body.role,
        )
        .await?;

    Ok((StatusCode::CREATED, Json(DoctorResponse::from(doctor))))
}

/// `GET /api/v1/doctors/:id`
pub async fn get_doctor(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<DoctorResponse>, AppError> {
    let doctor = state.doctor_service.get(id).await?;
    Ok(Json(DoctorResponse::from(doctor)))
}

/// `GET /api/v1/doctors`
pub async fn list_doctors(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<DoctorResponse>>, AppError> {
    let doctors = state
        .doctor_service
        .list(query.limit, query.offset)
        .await?;

    Ok(Json(
        doctors.into_iter().map(DoctorResponse::from).collect(),
    ))
}

/// `PATCH /api/v1/doctors/:id`
///
/// **Authorization:** admin can edit any doctor; a doctor can edit their
/// own profile. Enforced inline.
pub async fn update_doctor(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<UpdateDoctorRequest>,
) -> Result<Json<DoctorResponse>, AppError> {
    if claims.role != "admin" && claims.sub != id {
        return Err(CoreError::Forbidden(
            "you may only edit your own profile".into(),
        )
            .into());
    }

    let doctor = state
        .doctor_service
        .update(
            Some(claims.sub),
            id,
            body.full_name.as_deref(),
            body.specialty.as_deref(),
        )
        .await?;

    Ok(Json(DoctorResponse::from(doctor)))
}

/// `DELETE /api/v1/doctors/:id`
///
/// **Authorization:** admin only.
pub async fn delete_doctor(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    require_role(&claims, "admin")?;

    state.doctor_service.delete(Some(claims.sub), id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Small helper that returns `Forbidden` if the caller's role does not
/// match. This is deliberately a free function rather than a method on
/// `Claims` so that the authorization policy stays visible at the call
/// site — when we move to middleware, we will delete these calls and
/// replace them with a layer.
fn require_role(claims: &Claims, role: &str) -> Result<(), AppError> {
    if claims.role != role {
        return Err(CoreError::Forbidden(format!(
            "requires role: {role}"
        ))
            .into());
    }
    Ok(())
}