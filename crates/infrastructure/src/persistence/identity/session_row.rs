use chrono::{DateTime, Utc};
use domain::identity::{Session, SessionEndReason, SessionId, UserId};

#[derive(Debug, sqlx::FromRow)]
pub struct SessionRow {
    pub id: i64,
    pub user_id: i64,
    pub started_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub ended_reason: Option<String>,
}

impl SessionRow {
    pub fn into_entity(self) -> Session {
        Session {
            id: SessionId::new(self.id),
            user_id: UserId::new(self.user_id),
            started_at: self.started_at,
            last_seen_at: self.last_seen_at,
            ended_at: self.ended_at,
            ended_reason: self.ended_reason.as_deref().and_then(reason_from),
        }
    }
}

/// An unrecognised value means the row was written by a newer release or
/// edited by hand. Reading it as "ended, reason unknown" keeps the session
/// correctly closed rather than failing the whole query over a label.
fn reason_from(stored: &str) -> Option<SessionEndReason> {
    match stored {
        "logged_out" => Some(SessionEndReason::LoggedOut),
        "expired" => Some(SessionEndReason::Expired),
        "token_reused" => Some(SessionEndReason::TokenReused),
        "password_changed" => Some(SessionEndReason::PasswordChanged),
        "account_disabled" => Some(SessionEndReason::AccountDisabled),
        "revoked_by_admin" => Some(SessionEndReason::RevokedByAdmin),
        _ => None,
    }
}
