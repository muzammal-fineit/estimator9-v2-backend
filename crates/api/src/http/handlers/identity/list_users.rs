use axum::Json;
use axum::extract::State;
use domain::identity::PermissionName;

use crate::http::dto::error::ErrorResponse;
use crate::http::dto::identity::UserResponse;
use crate::http::{ApiError, AuthenticatedUser, ClientContext};
use crate::state::AppState;

/// List all users.
///
/// Ordered by id ascending. Not paginated — the user table is small by design;
/// pagination arrives if and when it stops being.
#[utoipa::path(
    get,
    path = "/users",
    tag = "identity",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Every user, oldest first", body = [UserResponse]),
        (status = 401, description = "Missing, malformed or expired access token", body = ErrorResponse),
        (status = 403, description = "Authenticated but lacks users.read — retrying will not help", body = ErrorResponse),
        (status = 429, description = "Too many requests; see Retry-After", body = ErrorResponse),
        (status = 503, description = "The data store is unavailable; retry", body = ErrorResponse),
    ),
)]
pub async fn list_users(
    State(state): State<AppState>,
    AuthenticatedUser(claims): AuthenticatedUser,
    ClientContext(context): ClientContext,
) -> Result<Json<Vec<UserResponse>>, ApiError> {
    state
        .identity
        .authorize
        .require(&claims, PermissionName::USERS_READ, context)
        .await?;

    let users = state.identity.list_users.execute().await?;

    Ok(Json(users.iter().map(UserResponse::from).collect()))
}
