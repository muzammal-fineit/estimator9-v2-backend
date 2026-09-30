use async_trait::async_trait;

use super::{Password, PasswordHash};

/// Hashing and verification, as the domain needs them.
///
/// Async even though hashing is pure CPU: a deliberately slow hash would stall
/// an async worker thread for its whole duration, so the adapter needs the
/// freedom to move the work off the runtime. Making that the port's shape means
/// no caller has to remember to.
#[async_trait]
pub trait PasswordHasher: Send + Sync + 'static {
    async fn hash(&self, password: &Password) -> Result<PasswordHash, HashingError>;

    /// Whether `password` matches `hash`.
    ///
    /// Returns `Ok(false)` for a mismatch rather than an error — a wrong
    /// password is an expected outcome, not a failure of the hasher.
    async fn verify(&self, password: &Password, hash: &PasswordHash) -> Result<bool, HashingError>;
}

#[derive(Debug, thiserror::Error)]
pub enum HashingError {
    /// The stored hash could not be parsed — a corrupt or foreign-format row.
    #[error("stored password hash is unreadable")]
    MalformedHash,

    #[error("password hashing failed")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl HashingError {
    pub fn backend(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Backend(Box::new(source))
    }
}
