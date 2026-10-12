//! # Doctor domain model
//!
//! A doctor is a practitioner who can be assigned appointments. Unlike
//! patients, doctors are also *users* of the system — they authenticate and
//! have access to specific records based on role.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A registered doctor.

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Doctor {
    pub id: Uuid,
    pub email: String,
    pub full_name: String,
    pub specialty: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}