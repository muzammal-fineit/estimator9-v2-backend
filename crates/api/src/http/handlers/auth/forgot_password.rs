use application::ApplicationError;
use axum::extract::State;
use axum::http::StatusCode;
use chrono::Utc;
use domain::identity::Email;

use crate::http::dto::auth::ForgotPasswordRequest;
use crate::http::dto::error::ErrorResponse;
use crate::http::{ApiError, ApiJson, ClientContext};
use crate::state::AppState;

/// Ask for a password reset link.
///
/// **Always returns 204**, whether or not the address belongs to an account.
/// Reporting "no such user" would turn this into a way to ask whether someone
/// holds an account here — a leak that needs no credentials at all.
///
/// Rate limited: without it, this endpoint sends mail to any address a caller
/// names, as often as they like.
#[utoipa::path(
    post,
    path = "/auth/password/forgot",
    tag = "auth",
    request_body = ForgotPasswordRequest,
    responses(
        (status = 204, description = "Accepted. A link is sent only if the address has an active account, and the response is identical either way"),
        (status = 400, description = "The body is not valid JSON", body = ErrorResponse),
        (status = 422, description = "That is not a valid email address", body = ErrorResponse),
        (status = 429, description = "Too many requests; see Retry-After", body = ErrorResponse),
        (status = 503, description = "The data store is unavailable; retry", body = ErrorResponse),
    ),
)]
pub async fn forgot_password(
    State(state): State<AppState>,
    ClientContext(context): ClientContext,
    ApiJson(body): ApiJson<ForgotPasswordRequest>,
) -> Result<StatusCode, ApiError> {
    // A malformed address is a 422 — it cannot reveal anything, because
    // nothing has been looked up.
    let email = Email::parse(body.email).map_err(ApplicationError::Invalid)?;

    state
        .identity
        .request_password_reset
        .execute(email, context, Utc::now())
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
