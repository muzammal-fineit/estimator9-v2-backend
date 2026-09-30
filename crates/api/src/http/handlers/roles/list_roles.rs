use axum::Json;
use axum::extract::State;
use domain::identity::PermissionName;

use crate::http::dto::error::ErrorResponse;
use crate::http::dto::roles::RoleResponse;
use crate::http::{ApiError, AuthenticatedUser, ClientContext};
use crate::state::AppState;

/// List the roles that can be assigned.
///
/// Includes `superadmin`, flagged, so a UI can show it as unassignable rather
/// than offering it and taking a 403.
#[utoipa::path(
    get,
    path = "/roles",
    tag = "roles",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Every role, alphabetically", body = [RoleResponse]),
        (status = 401, description = "Missing, malformed or expired access token", body = ErrorResponse),
        (status = 403, description = "Lacks roles.read", body = ErrorResponse),
        (status = 503, description = "The data store is unavailable; retry", body = ErrorResponse),
    ),
)]
pub async fn list_roles(
    State(state): State<AppState>,
    AuthenticatedUser(claims): AuthenticatedUser,
    ClientContext(context): ClientContext,
) -> Result<Json<Vec<RoleResponse>>, ApiError> {
    state
        .identity
        .authorize
        .require(&claims, PermissionName::ROLES_READ, context)
        .await?;

    let roles = state.identity.list_roles.roles().await?;

    Ok(Json(roles.iter().map(RoleResponse::from).collect()))
}
