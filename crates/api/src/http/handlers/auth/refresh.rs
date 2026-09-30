use application::ApplicationError;
use axum::extract::State;
use axum::http::HeaderMap;
use chrono::Utc;

use super::{cookie_token, session_response};
use crate::http::dto::auth::SessionResponse;
use crate::http::dto::error::ErrorResponse;
use crate::http::{ApiError, ClientContext};
use crate::state::AppState;

/// Exchange the refresh cookie for a new access token.
///
/// The old refresh token is revoked and replaced, so each one is usable
/// exactly once. Presenting an already-used token ends the whole session — see
/// [`application::identity::RefreshSession`].
///
/// Takes no request body: the credential is the httpOnly cookie, which script
/// cannot read and the browser attaches automatically.
#[utoipa::path(
    post,
    path = "/auth/refresh",
    tag = "auth",
    responses(
        (status = 200, description = "A new session", body = SessionResponse),
        (status = 401, description = "Missing, expired, or already-used refresh token. Reuse ends the whole session", body = ErrorResponse),
        (status = 503, description = "The data store is unavailable; retry", body = ErrorResponse),
    ),
)]
pub async fn refresh(
    State(state): State<AppState>,
    ClientContext(context): ClientContext,
    headers: HeaderMap,
) -> Result<axum::response::Response, ApiError> {
    let secret = cookie_token(&headers).ok_or(ApplicationError::Unauthenticated)?;

    let session = state
        .identity
        .refresh_session
        .execute(secret, context, Utc::now())
        .await?;

    Ok(session_response(session, &state.cookies))
}
