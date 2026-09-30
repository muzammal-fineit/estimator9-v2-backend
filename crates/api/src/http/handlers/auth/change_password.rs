use application::ApplicationError;
use application::identity::PasswordChange;
use axum::extract::State;
use chrono::Utc;
use domain::identity::Password;

use super::session_response;
use crate::http::dto::auth::{ChangePasswordRequest, SessionResponse};
use crate::http::dto::error::ErrorResponse;
use crate::http::{ApiError, ApiJson, AuthenticatedUser, ClientContext};
use crate::state::AppState;

/// Change your own password.
///
/// Requires the current password even though the caller already holds a valid
/// token — a token is something a borrowed laptop already has, and this is the
/// operation that would lock its owner out.
///
/// Every session ends, including this one, and a fresh session is returned. So
/// the caller stays signed in and every other device is signed out, which is
/// the point of changing a password you think someone else may know.
#[utoipa::path(
    post,
    path = "/auth/password/change",
    tag = "auth",
    security(("bearer" = [])),
    request_body = ChangePasswordRequest,
    responses(
        (status = 200, description = "Changed; a replacement session is returned and all others are ended", body = SessionResponse),
        (status = 400, description = "The body is not valid JSON", body = ErrorResponse),
        (status = 401, description = "Missing token, or the current password is wrong", body = ErrorResponse),
        (status = 422, description = "The new password does not meet policy", body = ErrorResponse),
        (status = 429, description = "Too many attempts; see Retry-After", body = ErrorResponse),
        (status = 503, description = "The data store is unavailable; retry", body = ErrorResponse),
    ),
)]
pub async fn change_password(
    State(state): State<AppState>,
    AuthenticatedUser(claims): AuthenticatedUser,
    ClientContext(context): ClientContext,
    ApiJson(body): ApiJson<ChangePasswordRequest>,
) -> Result<axum::response::Response, ApiError> {
    let change = PasswordChange {
        // Parsed with the same policy as the replacement. A current password
        // that predates the policy would fail here rather than at the hash
        // comparison — but it could not have been set through this API.
        current: Password::parse(body.current_password).map_err(ApplicationError::Invalid)?,
        replacement: Password::parse(body.new_password).map_err(ApplicationError::Invalid)?,
    };

    let session = state
        .identity
        .change_password
        .execute(&claims, change, context, Utc::now())
        .await?;

    Ok(session_response(session, &state.cookies))
}
