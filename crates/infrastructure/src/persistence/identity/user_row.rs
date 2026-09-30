use chrono::{DateTime, Utc};
use domain::identity::{Email, LoginControls, PasswordHash, Role, User, UserId};
use domain::shared::RepositoryError;

/// The shape of a `users` row. Separate from the `User` entity on purpose:
/// the table is free to change without the domain noticing, and the entity is
/// free to keep its fields private.
#[derive(Debug, sqlx::FromRow)]
pub struct UserRow {
    pub id: i64,
    pub email: String,
    pub name: String,
    pub password_hash: String,
    pub is_active: bool,
    pub failed_login_attempts: i32,
    pub locked_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl UserRow {
    /// Roles arrive separately because they come from a batch query over all
    /// the users being loaded, not from this row.
    pub fn into_entity(self, roles: Vec<Role>) -> Result<User, RepositoryError> {
        let email = Email::parse(self.email)?;
        let password_hash = PasswordHash::parse(self.password_hash)?;

        // Postgres has no unsigned integer type, so the column is `integer`.
        // A negative value would mean someone edited the row by hand; treat it
        // as zero rather than failing the whole read.
        let login_controls = LoginControls::rehydrate(
            self.failed_login_attempts.max(0).unsigned_abs(),
            self.locked_until,
        );

        Ok(User::rehydrate(
            UserId::new(self.id),
            email,
            self.name,
            password_hash,
            self.is_active,
            roles,
            login_controls,
            self.created_at,
            self.updated_at,
        )?)
    }
}
