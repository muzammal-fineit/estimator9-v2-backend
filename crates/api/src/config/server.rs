use super::{ConfigError, env};

#[derive(Debug)]
pub struct ServerConfig {
    /// Which interface to listen on.
    ///
    /// `0.0.0.0` is right for development — WSL needs it so a Windows browser
    /// can reach the API. In production it must be `127.0.0.1`: the rate
    /// limiter and the audit trail both take the caller's address from
    /// `X-Real-IP`, which is only trustworthy while nginx is the sole path in.
    /// Reachable directly, anyone can set that header to whatever they like.
    pub bind_address: String,

    pub port: u16,
    pub enable_docs: bool,
}

impl ServerConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            bind_address: env::optional("BIND_ADDRESS", "0.0.0.0"),

            // Parsed at startup rather than carried as a string to the bind
            // call: `PORT=800O` should fail in the first millisecond with a
            // message naming PORT, not later with an opaque address error.
            port: env::parsed("PORT", 3000u16, "expected a port between 1 and 65535")?,
            enable_docs: env::flag("ENABLE_DOCS", true),
        })
    }
}
