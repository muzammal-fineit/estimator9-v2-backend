use super::{ConfigError, env};

/// How the refresh cookie is written.
#[derive(Debug)]
pub struct CookieConfig {
    /// `Secure` means the browser only sends it over HTTPS. Must be true in
    /// any real deployment; the escape hatch exists because plain-http
    /// development over something other than `localhost` would otherwise never
    /// receive the cookie at all.
    pub secure: bool,

    /// Matches the refresh token's own lifetime — a cookie outliving the token
    /// it carries just produces confusing 401s.
    pub max_age_seconds: i64,
}

impl CookieConfig {
    pub fn from_env(refresh_ttl_days: i64) -> Result<Self, ConfigError> {
        Ok(Self {
            secure: env::flag("COOKIE_SECURE", true),
            max_age_seconds: refresh_ttl_days * 24 * 60 * 60,
        })
    }
}
