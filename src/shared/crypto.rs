//! # Cryptographic helpers
//!
//! Password hashing and verification using **Argon2id**, the winner of the
//! Password Hashing Competition and the current OWASP recommendation for
//! password storage.
//!
//! ## Why Argon2id?
//!
//! - **Memory-hard:** resistant to GPU and ASIC brute-force attacks.
//! - **Side-channel resistant:** the `id` variant combines data-independent
//!   and data-dependent memory access, balancing security against
//!   time-memory trade-off attacks.
//! - **Tunable:** the cost parameters (time, memory, parallelism) can be
//!   adjusted as hardware improves.
//!
//! ## References
//!
//! - [`argon2` crate](https://docs.rs/argon2/latest/argon2/)
//! - [OWASP Password Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)

use argon2::{
    password_hash::{
        rand_core::OsRng,
        PasswordHash,
        PasswordHasher,
        PasswordVerifier,
        SaltString,
    },
    Argon2,
};

use crate::core::errors::CoreError;

/// Hashes a plaintext password using Argon2id with a randomly generated salt.
///
/// The returned string is a PHC-formatted hash that includes the algorithm,
/// parameters, salt, and hash digest. It is safe to store in a database.
///
/// # Errors
///
/// Returns [`CoreError::Internal`] if hashing fails. In practice this is
/// extremely rare; it usually indicates a bug in the `argon2` crate or a
/// system-level issue (e.g., out of memory).
///
/// # Example
///
/// ```no_run
/// use hospital_register::shared::crypto::hash_password;
///
/// let hash = hash_password("correct horse battery staple").unwrap();
/// assert!(hash.starts_with("$argon2id$"));
/// ```
pub fn hash_password(password: &str) -> Result<String, CoreError> {
    // A fresh random salt is generated for every password. This ensures
    // that two users with the same password have different hashes.
    let salt = SaltString::generate(&mut OsRng);

    // `Argon2::default()` uses the `argon2id` variant with parameters
    // recommended by the crate authors. These are suitable for most
    // applications; tune them if you have specific latency budgets.
    let argon2 = Argon2::default();

    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| {
            CoreError::Internal(format!("password hashing failed: {error}"))
        })
}

/// Verifies a plaintext password against a previously stored Argon2 hash.
///
/// Returns `Ok(true)` if the password matches, `Ok(false)` if it does not.
///
/// # Errors
///
/// Returns [`CoreError::Internal`] if the stored hash is malformed. This
/// should be treated as a data-integrity error, not an authentication
/// failure.
///
/// # Example
///
/// ```no_run
/// use hospital_register::shared::crypto::{hash_password, verify_password};
///
/// let hash = hash_password("hunter2").unwrap();
/// assert!(verify_password("hunter2", &hash).unwrap());
/// assert!(!verify_password("wrong", &hash).unwrap());
/// ```
pub fn verify_password(
    password: &str,
    hash: &str,
) -> Result<bool, CoreError> {
    // Parse the stored PHC string. If it is malformed, we surface a
    // distinct error rather than pretending the password was wrong.
    let parsed_hash = PasswordHash::new(hash).map_err(|error| {
        CoreError::Internal(format!("invalid password hash: {error}"))
    })?;

    // `verify_password` performs a constant-time comparison internally,
    // so it is safe against timing attacks.
    let result = Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok();

    Ok(result)
}