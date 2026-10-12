//! # Appointment handlers
//!
//! HTTP handlers for the appointment resource.
//!
//! ## Design notes
//!
//! - **Listing** is done via query parameters on the collection endpoint
//!   (`GET /appointments?doctor_id=...`), not via nested resources. Nested
//!   resources (`GET /doctors/:id/appointments`) are a valid REST idiom,
//!   but they complicate routing in Axum: the nested router would have to
//!   be attached in `v1.rs`, and the doctor id would arrive via a path
//!   parameter that the appointment handler does not otherwise know about.
//!   Query parameters keep the router simple and the handler self-contained.
//!
//! - **Status changes** get their own sub-path (`PATCH /appointments/:id/status`)
//!   rather than being folded into a general `PATCH /appointments/:id`.
//!   Status transitions have domain rules (see
//!   [`crate::services::appointment_service::AppointmentService::update_status`])
//!   that do not apply to other fields, and separating them makes the audit
//!   trail clearer: every `PATCH .../status` produces one distinct kind of
//!   audit entry.
//!
//! ## Extractor ordering
//!
//! Axum processes extractors left to right. Two rules govern the order:
//!
//! 1. **`State` comes first.** It is always available and cheap.
//! 2. **Body-consuming extractors come last.** `ValidatedJson<T>` (and
//!    `Json<T>`, `Form<T>`) consumes the request body. Any extractor after
//!    it would see an already-drained request. So `ValidatedJson` is always
//!    the last argument.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Extension;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::api::dto::appointment_dto::{
    AppointmentResponse, CreateAppointmentRequest,
    UpdateAppointmentStatusRequest,
};
use crate::api::error::AppError;
use crate::api::extractors::validated_json::ValidatedJson;
use crate::app::state::AppState;
use crate::core::errors::CoreError;
use crate::services::auth_service::Claims;

/// Default page size when the client does not specify `limit`.
///
/// The service layer clamps whatever value arrives here, so a malicious
/// client cannot set this to `i64::MAX` and force a full table scan. The
/// default is a courtesy for well-behaved clients.
fn default_limit() -> i64 {
    20
}

/// Query parameters for `GET /api/v1/appointments`.
///
/// Exactly one of `patient_id` or `doctor_id` must be provided. Requesting
/// the entire appointments table is not supported: it would be unbounded,
/// slow, and of no practical use to any client. The handler validates this
/// and returns `400 Bad Request` if neither is set.
#[derive(Debug, Deserialize)]
pub struct ListAppointmentsQuery {
    pub patient_id: Option<Uuid>,
    pub doctor_id: Option<Uuid>,

    #[serde(default = "default_limit")]
    pub limit: i64,
}

/// `POST /api/v1/appointments`
///
/// Books a new appointment. The service layer enforces:
///
/// - the appointment must be in the future,
/// - the doctor must not already have an appointment within the conflict
///   window of the requested time.
pub async fn create_appointment(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    ValidatedJson(body): ValidatedJson<CreateAppointmentRequest>,
) -> Result<impl IntoResponse, AppError> {
    let appointment = state
        .appointment_service
        .book(
            Some(claims.sub),
            body.patient_id,
            body.doctor_id,
            body.scheduled_for,
            body.notes.as_deref(),
        )
        .await?;

    // `201 Created` is the correct response code for a newly-created
    // resource. It signals to the client that the body contains the
    // canonical representation, including the server-generated `id`.
    Ok((
        StatusCode::CREATED,
        Json(AppointmentResponse::from(appointment)),
    ))
}

/// `GET /api/v1/appointments/:id`
pub async fn get_appointment(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<AppointmentResponse>, AppError> {
    let appointment = state.appointment_service.get(id).await?;
    Ok(Json(AppointmentResponse::from(appointment)))
}

/// `GET /api/v1/appointments?patient_id=...` or `?doctor_id=...`
///
/// Lists appointments filtered by patient or doctor. Exactly one filter
/// must be supplied.
pub async fn list_appointments(
    State(state): State<AppState>,
    Query(query): Query<ListAppointmentsQuery>,
) -> Result<Json<Vec<AppointmentResponse>>, AppError> {
    // Reject the request if the client supplied neither filter, or both.
    // `match (a, b)` on a tuple is the idiomatic way to express "exactly
    // one of" in Rust — clearer than two nested `if` statements.
    let appointments = match (query.patient_id, query.doctor_id) {
        (Some(patient_id), None) => {
            state
                .appointment_service
                .list_for_patient(patient_id, query.limit)
                .await?
        }
        (None, Some(doctor_id)) => {
            state
                .appointment_service
                .list_for_doctor(doctor_id, query.limit)
                .await?
        }
        (Some(_), Some(_)) => {
            // Ambiguous: which filter wins? Reject rather than guess.
            return Err(CoreError::Validation(
                "specify exactly one of patient_id or doctor_id".into(),
            )
                .into());
        }
        (None, None) => {
            // Unbounded query: reject rather than scan the whole table.
            return Err(CoreError::Validation(
                "one of patient_id or doctor_id is required".into(),
            )
                .into());
        }
    };

    Ok(Json(
        appointments
            .into_iter()
            .map(AppointmentResponse::from)
            .collect(),
    ))
}

/// `PATCH /api/v1/appointments/:id/status`
///
/// Updates the lifecycle status of an appointment. The service layer
/// enforces the state transition rules — in particular, terminal statuses
/// (`completed`, `cancelled`, `no_show`) cannot be changed.
pub async fn update_appointment_status(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<UpdateAppointmentStatusRequest>,
) -> Result<Json<AppointmentResponse>, AppError> {
    let appointment = state
        .appointment_service
        .update_status(Some(claims.sub), id, body.status)
        .await?;

    Ok(Json(AppointmentResponse::from(appointment)))
}