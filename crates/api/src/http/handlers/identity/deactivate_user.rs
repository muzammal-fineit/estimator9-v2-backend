use axum::extract::{Path, State};
use axum::http::StatusCode;
use domain::identity::{PermissionName, UserId};

use crate::http::dto::error::ErrorResponse;
use crate::http::{ApiError, AuthenticatedUser, ClientContext};
use crate::state::AppState;

/// Deactivate a user account.
///
/// Not a delete: the row is retained so the audit trail keeps pointing at a
/// real account. Live sessions are closed and refresh tokens revoked in the
/// same transaction, so the account cannot obtain a new access token. One
/// already issued stays valid until it expires — at most the access-token
/// lifetime.
#[utoipa::path(
    delete,
    path = "/users/{id}",
    tag = "identity",
    security(("bearer" = [])),
    params(
        ("id" = i64, Path, description = "The account to deactivate", example = 2),
    ),
    responses(
        (status = 204, description = "Deactivated and its sessions ended"),
        (status = 401, description = "Missing, malformed or expired access token", body = ErrorResponse),
        (status = 403, description = "Lacks users.manage, or targeted yourself or a superadmin", body = ErrorResponse),
        (status = 404, description = "No such user, or it is already inactive", body = ErrorResponse),
        (status = 503, description = "The data store is unavailable; retry", body = ErrorResponse),
    ),
)]
pub async fn deactivate_user(
    State(state): State<AppState>,
    AuthenticatedUser(claims): AuthenticatedUser,
    ClientContext(context): ClientContext,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    state
        .identity
        .authorize
        .require(&claims, PermissionName::USERS_MANAGE, context.clone())
        .await?;

    state
        .identity
        .deactivate_user
        .execute(&claims, UserId::new(id), context)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
