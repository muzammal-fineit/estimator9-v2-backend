use axum::Json;
use axum::extract::State;

use crate::http::dto::error::ErrorResponse;
use crate::http::dto::identity::UserResponse;
use crate::http::{ApiError, AuthenticatedUser};
use crate::state::AppState;

/// The signed-in user.
///
/// Needs no permission — every authenticated caller may read their own
/// account, and Angular calls this on boot to render a header and decide which
/// navigation to show. It re-reads from the database rather than echoing the
/// token, so a name changed since login is current.
#[utoipa::path(
    get,
    path = "/auth/me",
    tag = "auth",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "The signed-in user", body = UserResponse),
        (status = 401, description = "Missing, malformed or expired access token", body = ErrorResponse),
        (status = 404, description = "The account no longer exists", body = ErrorResponse),
        (status = 503, description = "The data store is unavailable; retry", body = ErrorResponse),
    ),
)]
pub async fn me(
    State(state): State<AppState>,
    AuthenticatedUser(claims): AuthenticatedUser,
) -> Result<Json<UserResponse>, ApiError> {
    let user = state.identity.get_user.execute(claims.user_id).await?;

    Ok(Json(UserResponse::from(&user)))
}
