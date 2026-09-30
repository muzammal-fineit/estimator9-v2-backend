use async_trait::async_trait;

use super::{Email, LoginControls, PasswordHash, User, UserId};
use crate::shared::RepositoryError;

/// The port the identity context needs from persistence. The domain declares
/// it; `infrastructure` implements it; `application` only ever sees this trait.
#[async_trait]
pub trait UserRepository: Send + Sync + 'static {
    async fn all(&self) -> Result<Vec<User>, RepositoryError>;

    async fn find(&self, id: UserId) -> Result<Option<User>, RepositoryError>;

    /// Case-insensitive, matching the `users_email_lower_key` index —
    /// `Bank.User@x.com` and `bank.user@x.com` are one account, so a login
    /// must not depend on how the address was typed.
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, RepositoryError>;

    /// Replaces the hash and clears the login controls: a password change is a
    /// legitimate way out of a lockout, and leaving the counter set would lock
    /// the account again on the first typo after it.
    async fn set_password(&self, id: UserId, hash: &PasswordHash) -> Result<(), RepositoryError>;

    /// Persists the brute-force counters after an attempt.
    ///
    /// Separate from the rest of the user so a login attempt writes two
    /// columns rather than the whole row — this runs on every failed guess.
    async fn save_login_controls(
        &self,
        id: UserId,
        controls: &LoginControls,
    ) -> Result<(), RepositoryError>;
}
