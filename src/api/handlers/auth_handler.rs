//! Authentication handlers.

use axum::extract::State;
use axum::Json;

use crate::api::dto::auth_dto::{LoginRequest, LoginResponse};
use crate::api::error::AppError;
use crate::api::extractors::validated_json::ValidatedJson;
use crate::app::state::AppState;

/// `POST /api/v1/auth/login`
pub async fn login(
    State(state): State<AppState>,
    ValidatedJson(body): ValidatedJson<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    let token = state
        .auth_service
        .login(&body.email, &body.password)
        .await?;

    Ok(Json(LoginResponse {
        access_token: token,
        token_type: "Bearer".to_string(),
        expires_in: state.settings.jwt.access_ttl_minutes * 60,
    }))
}