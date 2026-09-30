use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::{Session, SessionEndReason, SessionId, UserId};
use crate::audit::RequestContext;
use crate::shared::RepositoryError;

#[async_trait]
pub trait SessionRepository: Send + Sync + 'static {
    /// Records a new session. Called once, when a password login succeeds —
    /// a refresh continues the session it was given rather than starting one.
    async fn start(
        &self,
        user: UserId,
        context: &RequestContext,
        at: DateTime<Utc>,
    ) -> Result<Session, RepositoryError>;

    async fn find(&self, id: SessionId) -> Result<Option<Session>, RepositoryError>;

    /// Advances `last_seen_at`. Called on refresh, not per request: a write on
    /// every call would cost more than the figure is worth.
    async fn touch(&self, id: SessionId, at: DateTime<Utc>) -> Result<(), RepositoryError>;

    /// Ends a session, keeping the first reason if it is already ended — the
    /// original cause is the one worth having.
    async fn end(
        &self,
        id: SessionId,
        reason: SessionEndReason,
        at: DateTime<Utc>,
    ) -> Result<(), RepositoryError>;

    /// Ends every live session a user has. Used by a password change and by an
    /// administrator disabling an account.
    async fn end_all_for_user(
        &self,
        user: UserId,
        reason: SessionEndReason,
        at: DateTime<Utc>,
    ) -> Result<(), RepositoryError>;
}
