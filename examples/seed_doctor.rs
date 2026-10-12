//! # Seed a doctor for local development
//!
//! Creates a doctor with a known email and password so the login flow can
//! be tested end-to-end.
//!
//! ## Why an example, not a test, not a binary?
//!
//! - **Not a test** — tests run in CI against throwaway databases. This
//!   script is meant to be run *manually* against your dev database.
//! - **Not a `[[bin]]`** — an extra binary bloats the release build and
//!   ships to production where it serves no purpose.
//! - **An example** — Cargo compiles it only when asked (`cargo run
//!   --example seed_doctor`), it is never part of `cargo build --release`,
//!   and it has full access to the crate's public API.
//!
//! ## Idempotency
//!
//! Running this twice does not crash. The second run sees a unique
//! violation on `email` and reports "already exists" instead of failing.
//! That makes the script safe to add to a `make seed` target later.
//!
//! ## References
//!
//! - [Cargo Book: Examples](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#examples)

use hospital_register::config::settings::Settings;
use hospital_register::core::errors::CoreError;
use hospital_register::core::traits::DoctorRepository;
use hospital_register::db::pool;
use hospital_register::db::repositories::doctor_repo::PostgresDoctorRepository;
use hospital_register::shared::crypto::hash_password;

/// Credentials the seed creates. Kept as constants so they are easy to
/// find and change. In a real project these would come from environment
/// variables or command-line arguments; for a dev seed, hardcoding is
/// honest and convenient.
const SEED_EMAIL: &str = "doc@hospital.test";
const SEED_PASSWORD: &str = "password123";
const SEED_NAME: &str = "Dr. Ada Lovelace";
const SEED_SPECIALTY: &str = "Cardiology";
const SEED_ROLE: &str = "admin";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load `.env` so DATABASE_URL and REDIS_URL are available. `ok()`
    // discards the "file not found" error — if `.env` is missing, the
    // variables may still be present in the real environment.
    dotenvy::dotenv().ok();

    // Same settings loader the server uses. If settings are wrong here,
    // they are wrong in the server too — which is exactly what we want:
    // the seed exercises the same config path.
    let settings = Settings::load()?;

    // `pool::build` also verifies connectivity to both Postgres and
    // Redis. If either is down, we fail fast with a clear error rather
    // than silently inserting nothing.
    let pools = pool::build(&settings).await?;

    // Construct the repository directly. We could instead build a full
    // `AppState` and use `DoctorService`, but that would spin up Redis
    // state and every other service for no reason. For a seed, the
    // repository is the right level of granularity.
    let repository = PostgresDoctorRepository::new(pools.pg.clone());

    // Hash the password. Same Argon2id path the register endpoint will
    // use. If this ever changes, both paths change together — that is
    // the point of routing hashing through `shared::crypto`.
    let password_hash = hash_password(SEED_PASSWORD)?;

    // Insert. Handle the "already exists" case idempotently.
    match repository
        .insert(
            SEED_EMAIL,
            &password_hash,
            SEED_NAME,
            SEED_SPECIALTY,
            SEED_ROLE,
        )
        .await
    {
        Ok(doctor) => {
            println!("Seeded doctor successfully.");
            println!();
            println!("  id:        {}", doctor.id);
            println!("  email:     {}", doctor.email);
            println!("  password:  {}", SEED_PASSWORD);
            println!("  role:      {}", doctor.role);
            println!();
            println!(
                "Use these credentials to POST /api/v1/auth/login."
            );
        }
        Err(CoreError::Conflict(_)) => {
            // This is the expected outcome on a second run. Not an error.
            println!(
                "Doctor with email '{SEED_EMAIL}' already exists. Skipping."
            );
            println!();
            println!(
                "If you forgot the password, delete the row and re-run:"
            );
            println!(
                "  docker exec -it hospital_postgres psql -U hospital \\"
            );
            println!("    -d hospital_register \\");
            println!(
                "    -c \"DELETE FROM doctors WHERE email = '{SEED_EMAIL}';\""
            );
        }
        Err(error) => {
            // Any other error (connection dropped mid-insert, check
            // violation on role, etc.) is genuinely unexpected.
            return Err(error.into());
        }
    }

    Ok(())
}