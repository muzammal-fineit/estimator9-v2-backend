use chrono::{DateTime, Utc};

use super::UserId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionId(i64);

impl SessionId {
    pub fn new(value: i64) -> Self {
        Self(value)
    }

    pub fn value(self) -> i64 {
        self.0
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Why a session stopped.
///
/// A closed set rather than free text: these are the values an operator
/// filters on when asked "why did everyone get logged out on Tuesday", and a
/// stray spelling makes that question unanswerable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEndReason {
    /// The user asked.
    LoggedOut,

    /// The refresh token reached its lifetime without being exchanged.
    Expired,

    /// An already-exchanged token was presented — see
    /// [`super::StoredRefreshToken::was_already_used`]. The alarming one.
    TokenReused,

    /// The password changed, so every session predating it ends.
    PasswordChanged,

    /// The account was deactivated.
    AccountDisabled,

    /// An administrator ended it deliberately.
    RevokedByAdmin,
}

impl SessionEndReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LoggedOut => "logged_out",
            Self::Expired => "expired",
            Self::TokenReused => "token_reused",
            Self::PasswordChanged => "password_changed",
            Self::AccountDisabled => "account_disabled",
            Self::RevokedByAdmin => "revoked_by_admin",
        }
    }

    /// Whether the session ended because something looked wrong, as opposed to
    /// the ordinary course of events. What a monitoring rule keys on.
    pub fn is_suspicious(self) -> bool {
        matches!(self, Self::TokenReused)
    }
}

/// One login, from the moment it succeeded until it ended.
///
/// Mutable state, deliberately separate from the append-only audit trail: a
/// session is revised as it lives, an audit entry never is.
#[derive(Debug, Clone)]
pub struct Session {
    pub id: SessionId,
    pub user_id: UserId,
    pub started_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub ended_reason: Option<SessionEndReason>,
}

impl Session {
    pub fn is_active(&self) -> bool {
        self.ended_at.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_reason_has_a_distinct_stored_form() {
        let all = [
            SessionEndReason::LoggedOut,
            SessionEndReason::Expired,
            SessionEndReason::TokenReused,
            SessionEndReason::PasswordChanged,
            SessionEndReason::AccountDisabled,
            SessionEndReason::RevokedByAdmin,
        ];

        let mut seen: Vec<&str> = all.iter().map(|r| r.as_str()).collect();
        seen.sort_unstable();
        let before = seen.len();
        seen.dedup();

        assert_eq!(seen.len(), before, "two reasons share a stored value");
    }

    #[test]
    fn only_token_reuse_is_treated_as_suspicious() {
        assert!(SessionEndReason::TokenReused.is_suspicious());
        assert!(!SessionEndReason::LoggedOut.is_suspicious());
        assert!(!SessionEndReason::Expired.is_suspicious());
    }
}
