//! # Authentication service
//!
//! Issues and verifies JWTs. Verifies passwords against Argon2 hashes.
//!
//! ## Token model
//!
//! We issue two tokens:
//!
//! - **Access token** — short-lived (15 min), sent with every request as
//!   `Authorization: Bearer <token>`. Stateless: no server-side lookup.
//! - **Refresh token** — long-lived (7 days), exchanged for a new access
//!   token. Stored in Redis with a TTL so it can be revoked server-side.
//!
//! For today's session we implement the access-token path fully, and stub
//! the refresh path. Refresh-token storage in Redis comes with the rate
//! limiter implementation.
//!
//! ## References
//!
//! - [`jsonwebtoken` crate](https://docs.rs/jsonwebtoken/latest/jsonwebtoken/)
//! - [RFC 7519 — JSON Web Token](https://datatracker.ietf.org/doc/html/rfc7519)

use std::sync::Arc;

use chrono::{Duration, Utc};
use jsonwebtoken::{
    decode, encode, DecodingKey, EncodingKey, Header, Validation,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::errors::CoreError;
use crate::core::traits::DoctorRepository;
use crate::shared::crypto::verify_password;

/// Claims carried inside the access token.
///
/// These are the fields clients cannot forge. Keep it minimal: anything
/// that can be looked up cheaply from a claim is fine to omit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject: the authenticated doctor's ID.
    pub sub: Uuid,

    /// Role: `doctor`, `admin`, or `receptionist`.
    pub role: String,

    /// Issued at (Unix timestamp, seconds).
    pub iat: i64,

    /// Expiry (Unix timestamp, seconds).
    pub exp: i64,
}

pub struct AuthService {
    doctor_repository: Arc<dyn DoctorRepository>,
    jwt_secret: String,
    access_ttl_minutes: i64,
}

impl AuthService {
    pub fn new(
        doctor_repository: Arc<dyn DoctorRepository>,
        jwt_secret: String,
        access_ttl_minutes: i64,
    ) -> Self {
        Self {
            doctor_repository,
            jwt_secret,
            access_ttl_minutes,
        }
    }

    /// Authenticates a doctor by email and password, returning a signed
    /// access token.
    ///
    /// # Security
    ///
    /// Whether the email is unknown or the password is wrong, we return the
    /// same error ([`CoreError::Unauthorized`]). This prevents attackers
    /// from enumerating valid emails by observing different error messages.
    pub async fn login(
        &self,
        email: &str,
        password: &str,
    ) -> Result<String, CoreError> {
        let doctor = self
            .doctor_repository
            .find_by_email_with_hash(email)
            .await?
            .ok_or_else(|| {
                CoreError::Unauthorized("invalid credentials".into())
            })?;

        // Verify the password. `verify_password` performs a constant-time
        // comparison internally.
        let password_ok =
            verify_password(password, &doctor.password_hash)?;
        if !password_ok {
            return Err(CoreError::Unauthorized(
                "invalid credentials".into(),
            ));
        }

        // Issue the access token.
        self.issue_access_token(doctor.id, &doctor.role)
    }

    /// Issues a signed JWT access token for the given subject and role.
    fn issue_access_token(
        &self,
        subject: Uuid,
        role: &str,
    ) -> Result<String, CoreError> {
        let now = Utc::now();
        let expires_at = now + Duration::minutes(self.access_ttl_minutes);

        let claims = Claims {
            sub: subject,
            role: role.to_string(),
            iat: now.timestamp(),
            exp: expires_at.timestamp(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
            .map_err(|error| {
                CoreError::Internal(format!("failed to encode JWT: {error}"))
            })
    }

    /// Verifies a signed JWT and returns its claims.
    ///
    /// The default `Validation` checks `exp` (expiry) automatically.
    pub fn verify_access_token(
        &self,
        token: &str,
    ) -> Result<Claims, CoreError> {
        let validation = Validation::default();
        let decoded = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &validation,
        )
            .map_err(|error| match error.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                    CoreError::Unauthorized("token expired".into())
                }
                _ => CoreError::Unauthorized("invalid token".into()),
            })?;

        Ok(decoded.claims)
    }
}