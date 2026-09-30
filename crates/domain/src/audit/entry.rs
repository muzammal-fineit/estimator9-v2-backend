use super::{Actor, AuditAction, AuditMetadata, RequestContext};
use crate::identity::SessionId;

/// One thing that happened, as the application understood it.
///
/// Built from the business operation rather than the HTTP request, which is why
/// it says `auth.login.failed` with a reason instead of `POST /auth/login 401`.
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub actor: Actor,
    pub action: AuditAction,

    /// True when the action was permitted by the superadmin bypass rather than
    /// by a granted permission. The column an auditor filters on first.
    pub via_superadmin: bool,

    /// What was acted upon, as (type, id) — `("user", "7")`.
    pub resource: Option<(&'static str, String)>,

    /// The login this happened during. Absent for events that precede one —
    /// a failed login has no session, which is the common case for the rows a
    /// security rule fires on.
    pub session: Option<SessionId>,

    pub metadata: AuditMetadata,
    pub context: RequestContext,
}

impl AuditEntry {
    pub fn new(action: AuditAction, actor: Actor, context: RequestContext) -> Self {
        Self {
            actor,
            action,
            via_superadmin: false,
            session: None,
            resource: None,
            metadata: AuditMetadata::new(),
            context,
        }
    }

    pub fn during(mut self, session: SessionId) -> Self {
        self.session = Some(session);
        self
    }

    pub fn via_superadmin(mut self, yes: bool) -> Self {
        self.via_superadmin = yes;
        self
    }

    pub fn about(mut self, kind: &'static str, id: impl std::fmt::Display) -> Self {
        self.resource = Some((kind, id.to_string()));
        self
    }

    pub fn with(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata = self.metadata.insert(key, value);
        self
    }
}
