use std::sync::Arc;

use chrono::{DateTime, Utc};
use domain::audit::{Actor, AuditAction, AuditEntry, AuditLogger, RequestContext};
use domain::identity::{
    RefreshTokenRepository, RefreshTokenSecret, SessionEndReason, SessionRepository,
    StoredRefreshToken, UserRepository,
};

use super::{EndSession, IssuedSession, SessionIssuer};
use crate::ApplicationError;
use crate::audit_trail::{actor_for, record};

/// Exchanges a refresh token for a new session, rotating it in the process.
///
/// Every rejection is the same [`ApplicationError::Unauthenticated`]: an
/// unknown token, an expired one, a replayed one and a disabled account must
/// look alike from outside. The trail records which it was.
pub struct RefreshSession {
    users: Arc<dyn UserRepository>,
    tokens: Arc<dyn RefreshTokenRepository>,
    sessions: Arc<dyn SessionRepository>,
    ending: Arc<EndSession>,
    issuer: Arc<SessionIssuer>,
    audit: Arc<dyn AuditLogger>,
}

impl RefreshSession {
    pub fn new(
        users: Arc<dyn UserRepository>,
        tokens: Arc<dyn RefreshTokenRepository>,
        sessions: Arc<dyn SessionRepository>,
        ending: Arc<EndSession>,
        issuer: Arc<SessionIssuer>,
        audit: Arc<dyn AuditLogger>,
    ) -> Self {
        Self {
            users,
            tokens,
            sessions,
            ending,
            issuer,
            audit,
        }
    }

    pub async fn execute(
        &self,
        secret: RefreshTokenSecret,
        context: RequestContext,
        now: DateTime<Utc>,
    ) -> Result<IssuedSession, ApplicationError> {
        let Some(stored) = self.tokens.find(&secret).await? else {
            return Err(ApplicationError::Unauthenticated);
        };

        // The reuse check comes first, before expiry: a replayed token that
        // has also expired is still a replay, and still means the session is
        // compromised.
        if stored.was_already_used() {
            return self.on_reuse(&stored, context, now).await;
        }

        if stored.is_expired_at(now) {
            self.ending
                .execute(stored.session_id, SessionEndReason::Expired, now)
                .await;

            return Err(ApplicationError::Unauthenticated);
        }

        // Gone and deactivated are the same answer: the account cannot be
        // used, so the session ends. Deactivation takes effect here even
        // though the access token it issued is still inside its own lifetime,
        // which is the bound on "how long does a disabled account keep
        // working".
        let user = match self.users.find(stored.user_id).await? {
            Some(user) if user.is_active() => user,
            _ => {
                self.ending
                    .execute(stored.session_id, SessionEndReason::AccountDisabled, now)
                    .await;

                return Err(ApplicationError::Unauthenticated);
            }
        };

        // Revoked before its successor exists: if issuing then fails, the
        // caller has to log in again, which is inconvenient. The other order
        // would leave a usable token behind after a failure, which is a
        // security bug. Inconvenience is the right way to fail.
        self.tokens.revoke(stored.id, now).await?;

        // The session continues; only the token rotates.
        self.sessions.touch(stored.session_id, now).await?;

        let session = self.issuer.issue(user, stored.session_id, now).await?;

        record(
            self.audit.as_ref(),
            AuditEntry::new(
                AuditAction::TOKEN_REFRESHED,
                actor_for(&session.user),
                context,
            )
            .during(session.session_id)
            .via_superadmin(session.user.is_superadmin()),
        )
        .await;

        Ok(session)
    }

    /// Someone presented a token that had already been exchanged.
    ///
    /// The whole session goes, not just this token: the thief holds one link
    /// of the chain and the victim holds another, and there is no way to tell
    /// which just called. Forcing both to log in again is the only safe answer.
    async fn on_reuse(
        &self,
        stored: &StoredRefreshToken,
        context: RequestContext,
        now: DateTime<Utc>,
    ) -> Result<IssuedSession, ApplicationError> {
        self.ending
            .execute(stored.session_id, SessionEndReason::TokenReused, now)
            .await;

        tracing::warn!(
            user_id = stored.user_id.value(),
            "refresh token reuse detected; the session has been ended"
        );

        let actor = match self.users.find(stored.user_id).await {
            Ok(Some(user)) => actor_for(&user),
            _ => Actor::Anonymous,
        };

        record(
            self.audit.as_ref(),
            AuditEntry::new(AuditAction::REFRESH_REUSED, actor, context).during(stored.session_id),
        )
        .await;

        Err(ApplicationError::Unauthenticated)
    }
}
