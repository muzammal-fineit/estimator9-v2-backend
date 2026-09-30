use axum::Json;
use axum::extract::State;
use domain::identity::PermissionName;

use crate::http::dto::error::ErrorResponse;
use crate::http::dto::roles::PermissionResponse;
use crate::http::{ApiError, AuthenticatedUser, ClientContext};
use crate::state::AppState;

/// List every permission the system checks for.
///
/// Reference data: it changes with a release, not through the API, because a
/// permission nothing in the code checks would be a label that grants nothing.
#[utoipa::path(
    get,
    path = "/permissions",
    tag = "roles",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Every permission, alphabetically", body = [PermissionResponse]),
        (status = 401, description = "Missing, malformed or expired access token", body = ErrorResponse),
        (status = 403, description = "Lacks roles.read", body = ErrorResponse),
        (status = 503, description = "The data store is unavailable; retry", body = ErrorResponse),
    ),
)]
pub async fn list_permissions(
    State(state): State<AppState>,
    AuthenticatedUser(claims): AuthenticatedUser,
    ClientContext(context): ClientContext,
) -> Result<Json<Vec<PermissionResponse>>, ApiError> {
    state
        .identity
        .authorize
        .require(&claims, PermissionName::ROLES_READ, context)
        .await?;

    let permissions = state.identity.list_roles.permissions().await?;

    Ok(Json(
        permissions.iter().map(PermissionResponse::from).collect(),
    ))
}
