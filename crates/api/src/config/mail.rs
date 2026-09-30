use chrono::Duration;

use super::{ConfigError, env};

/// Outbound mail, and the reset link it carries.
#[derive(Debug)]
pub struct MailConfig {
    /// Where the development transport appends messages. An SMTP adapter will
    /// replace it behind the same port without touching a use case.
    pub log_path: String,

    /// The page a reset link points at — the Angular route, not an API path.
    /// A token is useless to a human without a form to paste it into.
    pub reset_url: String,

    pub reset_ttl: Duration,
}

impl MailConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        // Short by design: a reset link sitting in a mailbox is a standing key
        // to the account.
        let minutes = env::parsed("PASSWORD_RESET_MINUTES", 60i64, "expected minutes")?;

        if !(5..=1440).contains(&minutes) {
            return Err(ConfigError::invalid(
                "PASSWORD_RESET_MINUTES",
                "must be between 5 and 1440",
            ));
        }

        Ok(Self {
            log_path: env::optional("MAIL_LOG_PATH", "logs/mail.log"),
            reset_url: env::optional("PASSWORD_RESET_URL", "http://localhost:4200/reset-password"),
            reset_ttl: Duration::minutes(minutes),
        })
    }
}
