use chrono::{DateTime, Utc};
use domain::identity::{RefreshTokenId, SessionId, StoredRefreshToken, UserId};

#[derive(Debug, sqlx::FromRow)]
pub struct RefreshTokenRow {
    pub id: i64,
    pub user_id: i64,
    pub session_id: i64,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

impl RefreshTokenRow {
    /// Infallible: every column maps to a newtype with no invariant of its own.
    /// The secret is absent by construction — it is not in the SELECT.
    pub fn into_entity(self) -> StoredRefreshToken {
        StoredRefreshToken {
            id: RefreshTokenId::new(self.id),
            user_id: UserId::new(self.user_id),
            session_id: SessionId::new(self.session_id),
            expires_at: self.expires_at,
            revoked_at: self.revoked_at,
        }
    }
}
