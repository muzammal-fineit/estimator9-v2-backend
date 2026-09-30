use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::{IssuedRefreshToken, RefreshTokenId, RefreshTokenSecret, SessionId, UserId};
use crate::shared::RepositoryError;

/// Storage for refresh tokens.
///
/// Generating the secret and hashing it belong to the adapter, not to a use
/// case: both are cryptographic details, and keeping the plaintext inside the
/// adapter means it exists in exactly one place, on exactly one code path.
#[async_trait]
pub trait RefreshTokenRepository: Send + Sync + 'static {
    /// Issues a token for a session. The returned secret is the only time it
    /// is available.
    async fn issue(
        &self,
        user: UserId,
        session: SessionId,
        expires_at: DateTime<Utc>,
    ) -> Result<IssuedRefreshToken, RepositoryError>;

    /// Looks a token up by its secret, **including revoked and expired ones**.
    ///
    /// Returning those rather than filtering them out is the point: a revoked
    /// token being presented is the reuse signal, and a query that hid it
    /// would turn an attack into an ordinary failed refresh.
    async fn find(
        &self,
        secret: &RefreshTokenSecret,
    ) -> Result<Option<super::StoredRefreshToken>, RepositoryError>;

    async fn revoke(&self, id: RefreshTokenId, at: DateTime<Utc>) -> Result<(), RepositoryError>;

    /// Revokes every unrevoked token belonging to a session.
    async fn revoke_session(
        &self,
        session: SessionId,
        at: DateTime<Utc>,
    ) -> Result<(), RepositoryError>;

    /// Revokes every session a user has. Used by logout-everywhere and, later,
    /// by a password change.
    async fn revoke_all_for_user(
        &self,
        user: UserId,
        at: DateTime<Utc>,
    ) -> Result<(), RepositoryError>;
}
