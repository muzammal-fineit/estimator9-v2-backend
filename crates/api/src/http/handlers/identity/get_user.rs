use axum::Json;
use axum::extract::{Path, State};
use domain::identity::{PermissionName, UserId};

use crate::http::dto::error::ErrorResponse;
use crate::http::dto::identity::UserResponse;
use crate::http::{ApiError, AuthenticatedUser, ClientContext};
use crate::state::AppState;

/// Fetch one user by id.
#[utoipa::path(
    get,
    path = "/users/{id}",
    tag = "identity",
    security(("bearer" = [])),
    params(
        ("id" = i64, Path, description = "The user's database id", example = 1),
    ),
    responses(
        (status = 200, description = "The user", body = UserResponse),
        (status = 401, description = "Missing, malformed or expired access token", body = ErrorResponse),
        (status = 403, description = "Authenticated but lacks users.read — retrying will not help", body = ErrorResponse),
        (status = 404, description = "No user has that id", body = ErrorResponse),
        (status = 429, description = "Too many requests; see Retry-After", body = ErrorResponse),
        (status = 503, description = "The data store is unavailable; retry", body = ErrorResponse),
    ),
)]
pub async fn get_user(
    State(state): State<AppState>,
    AuthenticatedUser(claims): AuthenticatedUser,
    ClientContext(context): ClientContext,
    Path(id): Path<i64>,
) -> Result<Json<UserResponse>, ApiError> {
    state
        .identity
        .authorize
        .require(&claims, PermissionName::USERS_READ, context)
        .await?;

    let user = state.identity.get_user.execute(UserId::new(id)).await?;

    Ok(Json(UserResponse::from(&user)))
}
