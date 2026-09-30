/// Why the process cannot start.
///
/// Note what these messages never contain: **the value**. `DATABASE_URL` holds a
/// password, and a startup error is the single most likely thing to end up in a
/// systemd journal, a CI log, or a screenshot pasted into a chat. Naming the
/// variable and the reason is enough to fix it; echoing the value is how
/// credentials leak.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("{name} is required but not set")]
    Missing { name: &'static str },

    #[error("{name} is set but invalid: {reason}")]
    Invalid {
        name: &'static str,
        reason: &'static str,
    },
}

impl ConfigError {
    pub fn invalid(name: &'static str, reason: &'static str) -> Self {
        Self::Invalid { name, reason }
    }
}
