use std::sync::Arc;

use chrono::{DateTime, Utc};
use domain::audit::{Actor, AuditAction, AuditEntry, AuditLogger, RequestContext};
use domain::identity::{
    RefreshTokenRepository, RefreshTokenSecret, SessionEndReason, UserRepository,
};

use super::EndSession;
use crate::ApplicationError;
use crate::audit_trail::{actor_for, record};

/// Ends a session.
///
/// Deliberately does **not** require an access token. A user whose access
/// token has already expired still needs to be able to log out, and refusing
/// would leave the refresh token alive — the opposite of what they asked for.
/// Presenting the cookie is authority enough to revoke it.
pub struct RevokeSession {
    users: Arc<dyn UserRepository>,
    tokens: Arc<dyn RefreshTokenRepository>,
    ending: Arc<EndSession>,
    audit: Arc<dyn AuditLogger>,
}

impl RevokeSession {
    pub fn new(
        users: Arc<dyn UserRepository>,
        tokens: Arc<dyn RefreshTokenRepository>,
        ending: Arc<EndSession>,
        audit: Arc<dyn AuditLogger>,
    ) -> Self {
        Self {
            users,
            tokens,
            ending,
            audit,
        }
    }

    /// Always succeeds.
    ///
    /// An unknown or already-revoked token still means "this caller wants to
    /// be logged out", and reporting an error would tell them whether the
    /// token was real — a small oracle, and useless to the honest caller whose
    /// cookie is being cleared either way.
    pub async fn execute(
        &self,
        secret: Option<RefreshTokenSecret>,
        context: RequestContext,
        now: DateTime<Utc>,
    ) -> Result<(), ApplicationError> {
        let Some(secret) = secret else {
            return Ok(());
        };

        let Some(stored) = self.tokens.find(&secret).await? else {
            return Ok(());
        };

        // Every token for the session, not just this one: logging out should
        // end the session, not leave its successor usable.
        self.ending
            .execute(stored.session_id, SessionEndReason::LoggedOut, now)
            .await;

        let actor = match self.users.find(stored.user_id).await {
            Ok(Some(user)) => actor_for(&user),
            _ => Actor::Anonymous,
        };

        record(
            self.audit.as_ref(),
            AuditEntry::new(AuditAction::LOGOUT, actor, context).during(stored.session_id),
        )
        .await;

        Ok(())
    }
}
