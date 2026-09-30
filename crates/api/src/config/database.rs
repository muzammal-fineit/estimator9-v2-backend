use super::{ConfigError, env};

pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

impl DatabaseConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let url = env::required("DATABASE_URL")?;

        if !(url.starts_with("postgres://") || url.starts_with("postgresql://")) {
            return Err(ConfigError::invalid(
                "DATABASE_URL",
                "must be a postgres:// or postgresql:// URL",
            ));
        }

        let max_connections =
            env::parsed("DB_MAX_CONNECTIONS", 10u32, "expected a positive number")?;

        if max_connections == 0 {
            return Err(ConfigError::invalid(
                "DB_MAX_CONNECTIONS",
                "must be at least 1",
            ));
        }

        Ok(Self {
            url,
            max_connections,
        })
    }
}

/// Hand-written so the password in `url` cannot reach a log through a
/// `tracing::info!(?config)` or a panic message.
impl std::fmt::Debug for DatabaseConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DatabaseConfig")
            .field("url", &"<redacted>")
            .field("max_connections", &self.max_connections)
            .finish()
    }
}
