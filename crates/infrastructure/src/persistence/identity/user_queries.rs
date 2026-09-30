//! The SQL behind [`super::PgUserRepository`].
//!
//! Separated from the repository so the mapping logic reads as logic and the
//! statements read as statements.
//!
//! sqlx 0.9 accepts only `&'static str`, and `concat!` cannot take a `const`,
//! so each query spells out its own column list. Adding a column to `users`
//! means editing every SELECT here — this file is the one place to look.

pub const SELECT_ALL: &str = "SELECT id, email, name, password_hash, is_active, \
                              failed_login_attempts, locked_until, created_at, updated_at \
                              FROM users ORDER BY id";

pub const SELECT_BY_ID: &str = "SELECT id, email, name, password_hash, is_active, \
                                failed_login_attempts, locked_until, created_at, updated_at \
                                FROM users WHERE id = $1";

/// `lower(email) = lower($1)` matches the `users_email_lower_key` index, so
/// this stays an index scan rather than degrading to a sequential one.
pub const SELECT_BY_EMAIL: &str = "SELECT id, email, name, password_hash, is_active, \
                                   failed_login_attempts, locked_until, created_at, updated_at \
                                   FROM users WHERE lower(email) = lower($1)";

/// One query for every user being loaded rather than one per user. `= ANY($1)`
/// takes the whole id list as a single bind.
pub const SELECT_ROLES_FOR_USERS: &str = "SELECT ur.user_id, r.id AS role_id, r.name, r.description \
     FROM user_roles ur \
     JOIN roles r ON r.id = ur.role_id \
     WHERE ur.user_id = ANY($1) \
     ORDER BY r.name";

/// Clearing the lockout alongside the hash is deliberate: a password change is
/// a legitimate way out of a lockout, and leaving the counter set would lock
/// the account again on the first typo afterwards.
pub const UPDATE_PASSWORD: &str = "UPDATE users \
                                   SET password_hash = $2, failed_login_attempts = 0, \
                                       locked_until = NULL, updated_at = now() \
                                   WHERE id = $1";

/// Two columns, not the whole row: this runs on every failed guess.
pub const UPDATE_LOGIN_CONTROLS: &str = "UPDATE users \
                                         SET failed_login_attempts = $2, locked_until = $3 \
                                         WHERE id = $1";
