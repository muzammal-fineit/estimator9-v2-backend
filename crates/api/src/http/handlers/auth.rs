//! Authentication endpoints, one file per operation.
//!
//! Shared below: reading the refresh cookie off a request, and building the
//! response that both `login` and `refresh` return. The same thing is handed
//! over in each case, so it must be built the same way in both — including the
//! part where the refresh token leaves only as a cookie.

pub mod change_password;
pub mod forgot_password;
pub mod login;
pub mod logout;
pub mod me;
pub mod refresh;
pub mod reset_password;
pub mod update_profile;

// Glob re-exports on purpose: `#[utoipa::path]` generates a `__path_<name>`
// struct alongside each handler, and `routes!(auth::login)` needs both in
// scope. Naming only the function compiles here and fails at the router.
pub use change_password::*;
pub use forgot_password::*;
pub use login::*;
pub use logout::*;
pub use me::*;
pub use refresh::*;
pub use reset_password::*;
pub use update_profile::*;

use application::identity::IssuedSession;
use axum::Json;
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use domain::identity::RefreshTokenSecret;

use crate::config::CookieConfig;
use crate::http::dto::auth::SessionResponse;
use crate::http::dto::identity::UserResponse;
use crate::http::refresh_cookie;

/// The body plus the rotated cookie.
///
/// The refresh token is attached as an httpOnly cookie and never placed in the
/// body, where script could read it.
fn session_response(session: IssuedSession, cookies: &CookieConfig) -> axum::response::Response {
    let body = Json(SessionResponse {
        access_token: session.token.as_str().to_owned(),
        token_type: "Bearer",
        expires_at: session.expires_at,
        user: UserResponse::from(&session.user),
    });

    match refresh_cookie::set(&session.refresh, cookies) {
        Some(header) => ([header], body).into_response(),
        None => body.into_response(),
    }
}

fn cookie_token(headers: &HeaderMap) -> Option<RefreshTokenSecret> {
    headers
        .get(axum::http::header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(refresh_cookie::read)
}
