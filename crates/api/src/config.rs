//! Everything the process reads from the environment, read once at startup so
//! no layer below has to reach for `std::env`.
//!
//! One module per concern. Phases still to come add `jwt`, `cors`, `mail` and
//! `rate_limit` here; each is a struct with its own `from_env`, listed once in
//! [`Config`]. Nothing else changes when a section is added.

pub mod cookie;
pub mod cors;
pub mod database;
pub mod env;
pub mod error;
pub mod jwt;
pub mod licensing;
pub mod mail;
pub mod rate_limit;
pub mod server;

pub use cookie::CookieConfig;
pub use cors::CorsConfig;
pub use database::DatabaseConfig;
pub use error::ConfigError;
pub use jwt::JwtConfig;
pub use licensing::LicensingConfig;
pub use mail::MailConfig;
pub use rate_limit::RateLimitConfig;
pub use server::ServerConfig;

#[derive(Debug)]
pub struct Config {
    pub database: DatabaseConfig,
    pub cookie: CookieConfig,
    pub cors: CorsConfig,
    pub jwt: JwtConfig,
    pub licensing: LicensingConfig,
    pub mail: MailConfig,
    pub rate_limit: RateLimitConfig,
    pub server: ServerConfig,
}

impl Config {
    /// Reads and validates every section.
    ///
    /// Fails on the first problem rather than collecting them all — an operator
    /// fixing an env file wants the next thing to fix, and a half-validated
    /// config is not a thing worth constructing.
    pub fn from_env() -> Result<Self, ConfigError> {
        let jwt = JwtConfig::from_env()?;

        Ok(Self {
            cookie: CookieConfig::from_env(jwt.refresh_ttl_days)?,
            cors: CorsConfig::from_env()?,
            database: DatabaseConfig::from_env()?,
            jwt,
            licensing: LicensingConfig::from_env()?,
            mail: MailConfig::from_env()?,
            rate_limit: RateLimitConfig::from_env()?,
            server: ServerConfig::from_env()?,
        })
    }
}
