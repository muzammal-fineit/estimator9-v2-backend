use super::{ConfigError, env};

/// Which browser origins may call this API.
#[derive(Debug)]
pub struct CorsConfig {
    /// Explicit origins. A wildcard is impossible here on purpose: browsers
    /// refuse `Access-Control-Allow-Origin: *` together with credentials, and
    /// the refresh cookie is a credential. Naming origins is not a limitation
    /// to work around — it is the only correct configuration.
    pub allowed_origins: Vec<String>,
}

impl CorsConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let raw = env::optional("CORS_ALLOWED_ORIGINS", "http://localhost:4200");

        let allowed_origins: Vec<String> = raw
            .split(',')
            .map(str::trim)
            .filter(|origin| !origin.is_empty())
            .map(str::to_owned)
            .collect();

        if allowed_origins.iter().any(|origin| origin == "*") {
            return Err(ConfigError::invalid(
                "CORS_ALLOWED_ORIGINS",
                "a wildcard cannot be used with credentials; list origins explicitly",
            ));
        }

        if allowed_origins.is_empty() {
            return Err(ConfigError::invalid(
                "CORS_ALLOWED_ORIGINS",
                "must name at least one origin",
            ));
        }

        Ok(Self { allowed_origins })
    }
}
