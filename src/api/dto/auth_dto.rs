//! Authentication request and response DTOs.

use serde::{Deserialize, Serialize};
use validator::Validate;

/// Request body for `POST /api/v1/auth/login`.
#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,

    #[validate(length(min = 8, max = 128))]
    pub password: String,
}

/// Response body for a successful login.
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
}