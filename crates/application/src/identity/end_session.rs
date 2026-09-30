use std::sync::Arc;

use chrono::{DateTime, Utc};
use domain::identity::{RefreshTokenRepository, SessionEndReason, SessionId, SessionRepository};

/// Ends a session: revokes its tokens and closes its row.
///
/// Shared because refresh and logout both do it, and doing only half is a
/// security bug in one direction (tokens live on) or a reporting bug in the
/// other (the row says alive when it is not).
pub struct EndSession {
    tokens: Arc<dyn RefreshTokenRepository>,
    sessions: Arc<dyn SessionRepository>,
}

impl EndSession {
    pub fn new(
        tokens: Arc<dyn RefreshTokenRepository>,
        sessions: Arc<dyn SessionRepository>,
    ) -> Self {
        Self { tokens, sessions }
    }

    /// Best effort, and deliberately so.
    ///
    /// Revoking the tokens is what actually stops access; closing the row is
    /// bookkeeping. Failing the request because the bookkeeping write failed
    /// would leave a caller unable to log out at all.
    pub async fn execute(&self, session: SessionId, reason: SessionEndReason, now: DateTime<Utc>) {
        if let Err(err) = self.tokens.revoke_session(session, now).await {
            tracing::error!(error = ?err, %session, "could not revoke session tokens");
        }

        if let Err(err) = self.sessions.end(session, reason, now).await {
            tracing::error!(error = ?err, %session, "could not close session row");
        }
    }
}
