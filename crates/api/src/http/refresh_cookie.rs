use axum::http::header::{HeaderValue, SET_COOKIE};
use domain::identity::RefreshTokenSecret;

use crate::config::CookieConfig;

/// The cookie name the browser will send back to `/auth/refresh`.
pub const NAME: &str = "estimator9_refresh";

/// Scoped to `/auth` so the refresh token is not attached to every API call.
/// A credential that travels on requests that cannot use it is a credential
/// with more chances to leak.
const PATH: &str = "/auth";

/// Builds the `Set-Cookie` header carrying a new refresh token.
///
/// `HttpOnly` is the point of the whole design: script cannot read it, so an
/// XSS hole in the Angular app cannot exfiltrate a long-lived credential. The
/// access token is deliberately *not* stored in a cookie — it belongs in
/// memory, where it dies with the tab.
///
/// `SameSite=Strict` because the API and the app are the same origin from the
/// browser's point of view once nginx fronts both; nothing legitimately
/// cross-site needs to refresh.
pub fn set(
    secret: &RefreshTokenSecret,
    cookies: &CookieConfig,
) -> Option<(&'static str, HeaderValue)> {
    let secure = if cookies.secure { "; Secure" } else { "" };

    let value = format!(
        "{NAME}={}; HttpOnly{secure}; SameSite=Strict; Path={PATH}; Max-Age={}",
        secret.expose(),
        cookies.max_age_seconds,
    );

    HeaderValue::from_str(&value)
        .ok()
        .map(|value| (SET_COOKIE.as_str(), value))
}

/// Builds the `Set-Cookie` header that removes it, for logout.
///
/// Attributes must match the cookie being replaced or the browser will keep
/// the original alongside the empty one.
pub fn clear(cookies: &CookieConfig) -> Option<(&'static str, HeaderValue)> {
    let secure = if cookies.secure { "; Secure" } else { "" };

    let value = format!("{NAME}=; HttpOnly{secure}; SameSite=Strict; Path={PATH}; Max-Age=0");

    HeaderValue::from_str(&value)
        .ok()
        .map(|value| (SET_COOKIE.as_str(), value))
}

/// Reads the refresh token out of a `Cookie` header.
pub fn read(header: &str) -> Option<RefreshTokenSecret> {
    header
        .split(';')
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, _)| *name == NAME)
        .map(|(_, value)| RefreshTokenSecret::new(value))
        .filter(|secret| !secret.expose().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(secure: bool) -> CookieConfig {
        CookieConfig {
            secure,
            max_age_seconds: 604_800,
        }
    }

    #[test]
    fn the_cookie_is_httponly_and_scoped_to_the_auth_path() {
        let secret = RefreshTokenSecret::new("abc123");
        let (_, value) = set(&secret, &config(true)).expect("header");
        let rendered = value.to_str().unwrap_or_default().to_owned();

        assert!(rendered.contains("HttpOnly"));
        assert!(rendered.contains("Secure"));
        assert!(rendered.contains("SameSite=Strict"));
        assert!(rendered.contains("Path=/auth"));
        assert!(rendered.contains("abc123"));
    }

    #[test]
    fn the_secure_flag_can_be_dropped_for_plain_http_development() {
        let secret = RefreshTokenSecret::new("abc123");
        let (_, value) = set(&secret, &config(false)).expect("header");

        assert!(!value.to_str().unwrap_or_default().contains("Secure"));
    }

    #[test]
    fn reads_its_own_cookie_out_of_a_crowded_header() {
        let header = format!("other=1; {NAME}=abc123; another=2");

        assert_eq!(
            read(&header).map(|s| s.expose().to_owned()),
            Some("abc123".to_owned())
        );
    }

    #[test]
    fn absent_or_empty_means_no_token() {
        assert!(read("other=1").is_none());
        assert!(read(&format!("{NAME}=")).is_none());
    }
}
