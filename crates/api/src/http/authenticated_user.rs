use application::ApplicationError;
use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use domain::identity::TokenClaims;

use super::ApiError;
use crate::state::AppState;

/// A caller with a valid access token.
///
/// Presence of this extractor in a handler's signature is what makes the
/// endpoint require authentication — there is no way to reach the body without
/// a verified token. It says nothing about *permissions*; those are checked in
/// the handler with `state.identity.authorize.require(...)`, deliberately
/// explicit so every check is greppable rather than hidden in an attribute.
pub struct AuthenticatedUser(pub TokenClaims);

impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let raw = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(bearer)
            .ok_or(ApplicationError::Unauthenticated)?;

        Ok(Self(state.identity.authorize.verify(raw)?))
    }
}

/// Case-insensitive on the scheme, because RFC 7235 says it is and clients
/// vary — Angular's `HttpHeaders` will send exactly what you give it.
fn bearer(header: &str) -> Option<&str> {
    let (scheme, token) = header.split_once(' ')?;

    scheme
        .eq_ignore_ascii_case("bearer")
        .then(|| token.trim())
        .filter(|token| !token.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_scheme_in_any_casing() {
        assert_eq!(bearer("Bearer abc"), Some("abc"));
        assert_eq!(bearer("bearer abc"), Some("abc"));
        assert_eq!(bearer("BEARER abc"), Some("abc"));
    }

    #[test]
    fn rejects_other_schemes_and_empty_tokens() {
        assert_eq!(bearer("Basic abc"), None);
        assert_eq!(bearer("Bearer"), None);
        assert_eq!(bearer("Bearer  "), None);
        assert_eq!(bearer("abc"), None);
    }
}
