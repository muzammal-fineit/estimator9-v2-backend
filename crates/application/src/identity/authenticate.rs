use std::sync::Arc;

use chrono::{DateTime, Utc};
use domain::audit::{Actor, AuditAction, AuditEntry, AuditLogger, LoginFailure, RequestContext};
use domain::identity::{Email, Password, SessionRepository};

use super::{CredentialCheck, IssuedSession, Rejected, SessionIssuer};
use crate::ApplicationError;
use crate::audit_trail::{actor_for, record};

pub struct Credentials {
    pub email: Email,
    pub password: Password,
}

/// Turns credentials into a session, and records what happened either way.
///
/// The credential rules themselves live in [`CredentialCheck`]; this type
/// decides what to do with its answer.
pub struct Authenticate {
    credentials: Arc<CredentialCheck>,
    sessions: Arc<dyn SessionRepository>,
    issuer: Arc<SessionIssuer>,
    audit: Arc<dyn AuditLogger>,
}

impl Authenticate {
    pub fn new(
        credentials: Arc<CredentialCheck>,
        sessions: Arc<dyn SessionRepository>,
        issuer: Arc<SessionIssuer>,
        audit: Arc<dyn AuditLogger>,
    ) -> Self {
        Self {
            credentials,
            sessions,
            issuer,
            audit,
        }
    }

    /// `now` is passed in rather than read here so expiry and lockout are
    /// testable without waiting.
    ///
    /// Every rejection but one returns [`ApplicationError::InvalidCredentials`]:
    /// a caller must not be able to tell an unknown address from a wrong
    /// password from a disabled account. The real reason goes to the audit
    /// trail. The exception is a locked account, reported only to someone who
    /// already supplied the correct password.
    pub async fn execute(
        &self,
        credentials: Credentials,
        context: RequestContext,
        now: DateTime<Utc>,
    ) -> Result<IssuedSession, ApplicationError> {
        match self.attempt(&credentials, &context, now).await {
            Ok(session) => {
                record(
                    self.audit.as_ref(),
                    AuditEntry::new(
                        AuditAction::LOGIN_SUCCEEDED,
                        actor_for(&session.user),
                        context,
                    )
                    .during(session.session_id)
                    .via_superadmin(session.user.is_superadmin()),
                )
                .await;

                Ok(session)
            }

            Err(Rejected::Refused(reason)) => {
                self.record_refusal(&credentials, context, reason, None)
                    .await;

                Err(ApplicationError::InvalidCredentials)
            }

            Err(Rejected::Locked { until }) => {
                self.record_refusal(
                    &credentials,
                    context,
                    LoginFailure::AccountLocked,
                    Some(until),
                )
                .await;

                Err(ApplicationError::AccountLocked { until })
            }

            Err(Rejected::Failed(err)) => Err(err),
        }
    }

    async fn record_refusal(
        &self,
        credentials: &Credentials,
        context: RequestContext,
        reason: LoginFailure,
        locked_until: Option<DateTime<Utc>>,
    ) {
        // The address is recorded even though the account may not exist — that
        // is precisely what makes a spray across many addresses visible
        // afterwards.
        let mut entry = AuditEntry::new(AuditAction::LOGIN_FAILED, Actor::Anonymous, context)
            .with("attempted_email", credentials.email.as_str())
            .with("reason", reason.as_str());

        if let Some(until) = locked_until {
            entry = entry.with("locked_until", until.to_rfc3339());
        }

        record(self.audit.as_ref(), entry).await;
    }

    async fn attempt(
        &self,
        credentials: &Credentials,
        context: &RequestContext,
        now: DateTime<Utc>,
    ) -> Result<IssuedSession, Rejected> {
        let user = self.credentials.execute(credentials, now).await?;

        // A password login always starts a new session rather than extending
        // whatever the caller may still be holding.
        let session = self.sessions.start(user.id(), context, now).await?;

        Ok(self.issuer.issue(user, session.id, now).await?)
    }
}
