use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::{IssuedPasswordReset, PasswordResetSecret, StoredPasswordReset, UserId};
use crate::shared::RepositoryError;

#[async_trait]
pub trait PasswordResetRepository: Send + Sync + 'static {
    /// Issues a token, superseding any the user already has outstanding.
    ///
    /// Superseding matters: without it, every "I forgot" click leaves another
    /// standing key to the account sitting in a mailbox.
    async fn issue(
        &self,
        user: UserId,
        expires_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<IssuedPasswordReset, RepositoryError>;

    /// Looks a token up **including used and expired ones**, so a replay is
    /// distinguishable from a token that never existed.
    async fn find(
        &self,
        secret: &PasswordResetSecret,
    ) -> Result<Option<StoredPasswordReset>, RepositoryError>;
}
