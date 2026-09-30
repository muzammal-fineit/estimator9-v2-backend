use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use chrono::Utc;

use super::cookie_token;
use crate::http::refresh_cookie;
use crate::http::{ApiError, ClientContext};
use crate::state::AppState;

/// End the session and clear the refresh cookie.
///
/// Needs no access token: a caller whose access token has already expired must
/// still be able to log out. Always succeeds — an unknown token still means
/// "log me out", and saying otherwise would reveal whether it was real.
#[utoipa::path(
    post,
    path = "/auth/logout",
    tag = "auth",
    responses(
        (status = 204, description = "Session ended and cookie cleared"),
    ),
)]
pub async fn logout(
    State(state): State<AppState>,
    ClientContext(context): ClientContext,
    headers: HeaderMap,
) -> Result<axum::response::Response, ApiError> {
    state
        .identity
        .revoke_session
        .execute(cookie_token(&headers), context, Utc::now())
        .await?;

    Ok(match refresh_cookie::clear(&state.cookies) {
        Some(header) => ([header], StatusCode::NO_CONTENT).into_response(),
        None => StatusCode::NO_CONTENT.into_response(),
    })
}
