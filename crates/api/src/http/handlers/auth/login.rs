use application::ApplicationError;
use application::identity::Credentials;
use axum::extract::State;
use chrono::Utc;
use domain::identity::{Email, Password};

use super::session_response;
use crate::http::dto::auth::{LoginRequest, SessionResponse};
use crate::http::dto::error::ErrorResponse;
use crate::http::{ApiError, ApiJson, ClientContext};
use crate::state::AppState;

/// Exchange an email and password for an access token.
///
/// Rejections are deliberately indistinguishable: an unknown address, a wrong
/// password and a disabled account all return the same 401. Anything else
/// would let a caller enumerate accounts.
///
/// The single exception is a locked account, which returns 423 — but only to a
/// caller who supplied the *correct* password, and so has already confirmed
/// the account exists. Someone guessing still sees 401.
#[utoipa::path(
    post,
    path = "/auth/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Authenticated", body = SessionResponse),
        (status = 400, description = "The body is not valid JSON", body = ErrorResponse),
        (status = 401, description = "Invalid credentials — indistinguishable from an unknown, locked or disabled account", body = ErrorResponse),
        (status = 415, description = "Content-Type is not application/json", body = ErrorResponse),
        (status = 422, description = "Email or password rejected by policy", body = ErrorResponse),
        (status = 423, description = "The password was correct but the account is temporarily locked after repeated failures", body = ErrorResponse),
        (status = 429, description = "Too many attempts; see Retry-After", body = ErrorResponse),
        (status = 503, description = "The data store is unavailable; retry", body = ErrorResponse),
    ),
)]
pub async fn login(
    State(state): State<AppState>,
    ClientContext(context): ClientContext,
    ApiJson(body): ApiJson<LoginRequest>,
) -> Result<axum::response::Response, ApiError> {
    // A malformed address or an under-length password is a 422, but it cannot
    // reveal anything about whether the account exists — nothing has been
    // looked up yet.
    let credentials = Credentials {
        email: Email::parse(body.email).map_err(ApplicationError::Invalid)?,
        password: Password::parse(body.password).map_err(ApplicationError::Invalid)?,
    };

    let session = state
        .identity
        .authenticate
        .execute(credentials, context, Utc::now())
        .await?;

    Ok(session_response(session, &state.cookies))
}
