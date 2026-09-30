use super::{ConfigError, env};

#[derive(Debug)]
pub struct LicensingConfig {
    /// Where the encrypted feature state is kept.
    ///
    /// Outside the repository and outside the database on purpose: it belongs
    /// to the binary, not to the client's data.
    pub state_path: String,
}

impl LicensingConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            state_path: env::optional("FEATURE_STATE_PATH", "var/features.bin"),
        })
    }
}
