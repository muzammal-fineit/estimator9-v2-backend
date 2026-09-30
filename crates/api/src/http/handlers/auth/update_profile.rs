use axum::extract::State;
use axum::http::StatusCode;

use crate::http::dto::auth::UpdateProfileRequest;
use crate::http::dto::error::ErrorResponse;
use crate::http::{ApiError, ApiJson, AuthenticatedUser, ClientContext};
use crate::state::AppState;

/// Change your own display name.
///
/// Name only: the email address is the login identifier, and changing it
/// unverified turns one typo into a locked-out account. That goes through an
/// administrator.
#[utoipa::path(
    patch,
    path = "/auth/me",
    tag = "auth",
    security(("bearer" = [])),
    request_body = UpdateProfileRequest,
    responses(
        (status = 204, description = "Updated"),
        (status = 400, description = "The body is not valid JSON", body = ErrorResponse),
        (status = 401, description = "Missing, malformed or expired access token", body = ErrorResponse),
        (status = 422, description = "The name is blank", body = ErrorResponse),
        (status = 503, description = "The data store is unavailable; retry", body = ErrorResponse),
    ),
)]
pub async fn update_profile(
    State(state): State<AppState>,
    AuthenticatedUser(claims): AuthenticatedUser,
    ClientContext(context): ClientContext,
    ApiJson(body): ApiJson<UpdateProfileRequest>,
) -> Result<StatusCode, ApiError> {
    state
        .identity
        .update_profile
        .execute(&claims, body.name, context)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
