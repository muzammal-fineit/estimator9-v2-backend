use application::ApplicationError;
use axum::extract::State;
use axum::http::StatusCode;
use chrono::Utc;
use domain::identity::{Password, PasswordResetSecret};

use crate::http::dto::auth::ResetPasswordRequest;
use crate::http::dto::error::ErrorResponse;
use crate::http::{ApiError, ApiJson, ClientContext};
use crate::state::AppState;

/// Redeem a reset link and set a new password.
///
/// Unknown, expired and already-used tokens all answer 401 alike — telling
/// them apart would say whether a link was ever real.
///
/// No session is returned: the user logs in afterwards. Signing them in here
/// would mean a link forwarded to the wrong inbox grants a live session rather
/// than only the chance to set a password.
#[utoipa::path(
    post,
    path = "/auth/password/reset",
    tag = "auth",
    request_body = ResetPasswordRequest,
    responses(
        (status = 204, description = "Password set and every session ended; sign in again"),
        (status = 400, description = "The body is not valid JSON", body = ErrorResponse),
        (status = 401, description = "The token is unknown, expired, or already used", body = ErrorResponse),
        (status = 422, description = "The new password does not meet policy", body = ErrorResponse),
        (status = 429, description = "Too many requests; see Retry-After", body = ErrorResponse),
        (status = 503, description = "The data store is unavailable; retry", body = ErrorResponse),
    ),
)]
pub async fn reset_password(
    State(state): State<AppState>,
    ClientContext(context): ClientContext,
    ApiJson(body): ApiJson<ResetPasswordRequest>,
) -> Result<StatusCode, ApiError> {
    let password = Password::parse(body.new_password).map_err(ApplicationError::Invalid)?;

    state
        .identity
        .reset_password
        .execute(
            PasswordResetSecret::new(body.token),
            password,
            context,
            Utc::now(),
        )
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
