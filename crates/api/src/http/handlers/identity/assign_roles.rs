use axum::extract::{Path, State};
use axum::http::StatusCode;
use domain::identity::{PermissionName, RoleName, UserId};

use crate::http::dto::error::ErrorResponse;
use crate::http::dto::identity::AssignRolesRequest;
use crate::http::{ApiError, ApiJson, AuthenticatedUser, ClientContext};
use crate::state::AppState;

/// Replace a user's roles.
///
/// The list is exhaustive: whatever is sent becomes the user's complete set,
/// and an empty list removes every role. A PATCH-style add/remove would make
/// concurrent edits silently lose one another.
///
/// `superadmin` cannot be granted or removed here, and nobody may change their
/// own roles — both would let a holder of `users.manage` escalate past every
/// other boundary.
#[utoipa::path(
    put,
    path = "/users/{id}/roles",
    tag = "identity",
    security(("bearer" = [])),
    params(
        ("id" = i64, Path, description = "The user whose roles are being replaced", example = 2),
    ),
    request_body = AssignRolesRequest,
    responses(
        (status = 204, description = "Roles replaced"),
        (status = 400, description = "The body is not valid JSON", body = ErrorResponse),
        (status = 401, description = "Missing, malformed or expired access token", body = ErrorResponse),
        (status = 403, description = "Lacks users.manage, or attempted to change own roles or a superadmin", body = ErrorResponse),
        (status = 404, description = "No such user, or a named role does not exist", body = ErrorResponse),
        (status = 422, description = "A role name is malformed", body = ErrorResponse),
        (status = 503, description = "The data store is unavailable; retry", body = ErrorResponse),
    ),
)]
pub async fn assign_roles(
    State(state): State<AppState>,
    AuthenticatedUser(claims): AuthenticatedUser,
    ClientContext(context): ClientContext,
    Path(id): Path<i64>,
    ApiJson(body): ApiJson<AssignRolesRequest>,
) -> Result<StatusCode, ApiError> {
    state
        .identity
        .authorize
        .require(&claims, PermissionName::USERS_MANAGE, context.clone())
        .await?;

    let roles = body
        .roles
        .into_iter()
        .map(RoleName::parse)
        .collect::<Result<Vec<_>, _>>()
        .map_err(application::ApplicationError::Invalid)?;

    state
        .identity
        .assign_roles
        .execute(&claims, UserId::new(id), roles, context)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
