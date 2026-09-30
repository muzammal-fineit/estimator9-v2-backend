use chrono::Duration;

use super::{ConfigError, env};

pub struct JwtConfig {
    pub secret: String,
    pub access_ttl: Duration,

    /// How long a refresh token lives. Far longer than the access token — it
    /// is the thing that stops a user re-entering their password hourly — and
    /// the reason it is stored hashed and rotated on every use.
    pub refresh_ttl: Duration,
    pub refresh_ttl_days: i64,
}

impl JwtConfig {
    /// Below this, an HS256 secret is brute-forceable offline by anyone
    /// holding one issued token.
    const MIN_SECRET_LENGTH: usize = 32;

    pub fn from_env() -> Result<Self, ConfigError> {
        let secret = env::required("JWT_SECRET")?;

        if secret.len() < Self::MIN_SECRET_LENGTH {
            return Err(ConfigError::invalid(
                "JWT_SECRET",
                "must be at least 32 characters; generate one with `openssl rand -base64 48`",
            ));
        }

        // Short by design: permissions are baked into the token, so this is
        // the window in which a revoked role still works.
        let minutes = env::parsed(
            "ACCESS_TOKEN_MINUTES",
            15i64,
            "expected a number of minutes",
        )?;

        if !(1..=1440).contains(&minutes) {
            return Err(ConfigError::invalid(
                "ACCESS_TOKEN_MINUTES",
                "must be between 1 and 1440",
            ));
        }

        let days = env::parsed("REFRESH_TOKEN_DAYS", 7i64, "expected a number of days")?;

        if !(1..=90).contains(&days) {
            return Err(ConfigError::invalid(
                "REFRESH_TOKEN_DAYS",
                "must be between 1 and 90",
            ));
        }

        Ok(Self {
            secret,
            access_ttl: Duration::minutes(minutes),
            refresh_ttl: Duration::days(days),
            refresh_ttl_days: days,
        })
    }
}

/// Redacted: the secret mints tokens for any account, superadmin included.
impl std::fmt::Debug for JwtConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtConfig")
            .field("secret", &"<redacted>")
            .field("access_ttl", &self.access_ttl)
            .field("refresh_ttl", &self.refresh_ttl)
            .finish()
    }
}
