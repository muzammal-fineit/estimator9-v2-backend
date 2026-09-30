use super::{ConfigError, env};

/// Per-address request quotas, per minute.
///
/// Two tiers for now. Login is far stricter than everything else because it is
/// the only endpoint where guessing repeatedly is worth an attacker's time.
#[derive(Debug)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub login_per_minute: u32,
    pub default_per_minute: u32,

    /// Per hour, not per minute. These endpoints send mail to whatever address
    /// a caller names, so the limit that matters is how many messages one
    /// address can cause in a day, not how fast they arrive.
    pub password_reset_per_hour: u32,
}

impl RateLimitConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let login_per_minute =
            env::parsed("RATE_LIMIT_LOGIN_PER_MINUTE", 5u32, "expected a number")?;
        let default_per_minute =
            env::parsed("RATE_LIMIT_DEFAULT_PER_MINUTE", 300u32, "expected a number")?;

        // A zero quota would mean "refuse everything", which is never what
        // someone means — they mean ENABLE_RATE_LIMIT=false.
        if login_per_minute == 0 {
            return Err(ConfigError::invalid(
                "RATE_LIMIT_LOGIN_PER_MINUTE",
                "must be at least 1; use ENABLE_RATE_LIMIT=false to turn limiting off",
            ));
        }

        if default_per_minute == 0 {
            return Err(ConfigError::invalid(
                "RATE_LIMIT_DEFAULT_PER_MINUTE",
                "must be at least 1; use ENABLE_RATE_LIMIT=false to turn limiting off",
            ));
        }

        let password_reset_per_hour = env::parsed(
            "RATE_LIMIT_PASSWORD_RESET_PER_HOUR",
            3u32,
            "expected a number",
        )?;

        if password_reset_per_hour == 0 {
            return Err(ConfigError::invalid(
                "RATE_LIMIT_PASSWORD_RESET_PER_HOUR",
                "must be at least 1; use ENABLE_RATE_LIMIT=false to turn limiting off",
            ));
        }

        Ok(Self {
            // On by default. An operator who wants it off has to say so.
            enabled: env::flag("ENABLE_RATE_LIMIT", true),
            login_per_minute,
            default_per_minute,
            password_reset_per_hour,
        })
    }
}

impl RateLimitConfig {
    /// For the `openapi` binary, which builds the router only to read the spec
    /// out of it and never serves a request.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            login_per_minute: 1,
            default_per_minute: 1,
            password_reset_per_hour: 1,
        }
    }
}
