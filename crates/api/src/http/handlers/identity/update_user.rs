use application::ApplicationError;
use application::identity::UserChanges;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use domain::identity::{Email, PermissionName, UserId};

use crate::http::dto::error::ErrorResponse;
use crate::http::dto::identity::UpdateUserRequest;
use crate::http::{ApiError, ApiJson, AuthenticatedUser, ClientContext};
use crate::state::AppState;

/// Change a user's name or address.
///
/// Partial: omit a field to leave it alone. An empty body is a 422 rather than
/// a silent success, since it is almost always a client bug.
///
/// Roles are not editable here — they have their own endpoint, with their own
/// escalation rails.
#[utoipa::path(
    patch,
    path = "/users/{id}",
    tag = "identity",
    security(("bearer" = [])),
    params(
        ("id" = i64, Path, description = "The account to change", example = 2),
    ),
    request_body = UpdateUserRequest,
    responses(
        (status = 204, description = "Updated"),
        (status = 400, description = "The body is not valid JSON", body = ErrorResponse),
        (status = 401, description = "Missing, malformed or expired access token", body = ErrorResponse),
        (status = 403, description = "Lacks users.manage, or targeted a superadmin", body = ErrorResponse),
        (status = 404, description = "No such user", body = ErrorResponse),
        (status = 409, description = "That email address is already in use", body = ErrorResponse),
        (status = 422, description = "Nothing to change, or the address is malformed", body = ErrorResponse),
        (status = 503, description = "The data store is unavailable; retry", body = ErrorResponse),
    ),
)]
pub async fn update_user(
    State(state): State<AppState>,
    AuthenticatedUser(claims): AuthenticatedUser,
    ClientContext(context): ClientContext,
    Path(id): Path<i64>,
    ApiJson(body): ApiJson<UpdateUserRequest>,
) -> Result<StatusCode, ApiError> {
    state
        .identity
        .authorize
        .require(&claims, PermissionName::USERS_MANAGE, context.clone())
        .await?;

    let changes = UserChanges {
        name: body.name,
        email: body
            .email
            .map(Email::parse)
            .transpose()
            .map_err(ApplicationError::Invalid)?,
    };

    state
        .identity
        .update_user
        .execute(&claims, UserId::new(id), changes, context)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
