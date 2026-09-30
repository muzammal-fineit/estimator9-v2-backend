use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use domain::audit::{Actor, AuditAction, AuditEntry, AuditLogger, RequestContext};
use domain::identity::{Email, PasswordResetRepository, UserRepository};
use domain::notification::{EmailMessage, EmailSender};

use crate::ApplicationError;
use crate::audit_trail::{actor_for, record};

/// Starts "I forgot my password".
///
/// **Always reports success**, whether or not the address belongs to an
/// account. Anything else turns this endpoint into a way to ask "does this
/// person bank here", which is a worse leak than most login oracles because it
/// needs no credentials at all.
pub struct RequestPasswordReset {
    users: Arc<dyn UserRepository>,
    resets: Arc<dyn PasswordResetRepository>,
    mail: Arc<dyn EmailSender>,
    audit: Arc<dyn AuditLogger>,
    ttl: Duration,
    reset_url: String,
}

impl RequestPasswordReset {
    pub fn new(
        users: Arc<dyn UserRepository>,
        resets: Arc<dyn PasswordResetRepository>,
        mail: Arc<dyn EmailSender>,
        audit: Arc<dyn AuditLogger>,
        ttl: Duration,
        reset_url: String,
    ) -> Self {
        Self {
            users,
            resets,
            mail,
            audit,
            ttl,
            reset_url,
        }
    }

    pub async fn execute(
        &self,
        email: Email,
        context: RequestContext,
        now: DateTime<Utc>,
    ) -> Result<(), ApplicationError> {
        let Some(user) = self.users.find_by_email(&email).await? else {
            // Recorded even though nothing was sent: a run of these against
            // addresses that do not exist is somebody probing the user list.
            record(
                self.audit.as_ref(),
                AuditEntry::new(
                    AuditAction::PASSWORD_RESET_REQUESTED,
                    Actor::Anonymous,
                    context,
                )
                .with("attempted_email", email.as_str())
                .with("outcome", "no_such_account"),
            )
            .await;

            return Ok(());
        };

        // A deactivated account gets no link. Silently, for the same reason:
        // saying so would confirm the address exists.
        if !user.is_active() {
            return Ok(());
        }

        let issued = self.resets.issue(user.id(), now + self.ttl, now).await?;

        let message = EmailMessage {
            to: user.email().clone(),
            subject: "Reset your Estimator9 password".to_owned(),
            body: format!(
                "Someone asked to reset the password for this address.\n\n\
                 {}?token={}\n\n\
                 The link stops working in {} minutes, or as soon as it is used.\n\
                 If this was not you, no action is needed — the password has not changed.",
                self.reset_url,
                issued.secret.expose(),
                self.ttl.num_minutes(),
            ),
        };

        // A failure here must not tell the caller anything, so it is logged
        // and swallowed: the response is identical either way.
        if let Err(err) = self.mail.send(message).await {
            tracing::error!(error = ?err, "could not send a password reset message");
        }

        record(
            self.audit.as_ref(),
            AuditEntry::new(
                AuditAction::PASSWORD_RESET_REQUESTED,
                actor_for(&user),
                context,
            )
            .about("user", user.id())
            .with("outcome", "sent"),
        )
        .await;

        Ok(())
    }
}
