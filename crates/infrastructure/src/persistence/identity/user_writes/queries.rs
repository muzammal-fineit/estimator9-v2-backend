//! The SQL behind the user-administration writes.

pub const SELECT_ROLE_IDS: &str = "SELECT id, name FROM roles WHERE name = ANY($1)";

pub const DELETE_USER_ROLES: &str = "DELETE FROM user_roles WHERE user_id = $1";
pub const INSERT_USER_ROLE: &str = "INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2)";

pub const INSERT_USER: &str = "INSERT INTO users (email, name, password_hash) \
                               VALUES ($1, $2, $3) RETURNING id";

/// `COALESCE` leaves a column alone when its parameter is null, so one
/// statement serves any combination of fields without building SQL at runtime.
pub const UPDATE_USER: &str = "UPDATE users \
                               SET name = COALESCE($2, name), \
                                   email = COALESCE($3, email), \
                                   updated_at = now() \
                               WHERE id = $1";

pub const CHANGE_PASSWORD: &str = "UPDATE users \
                                   SET password_hash = $2, failed_login_attempts = 0, \
                                       locked_until = NULL, updated_at = now() \
                                   WHERE id = $1";

/// Reason differs from the deactivation path, so the trail distinguishes
/// "logged out because the password changed" from "logged out because the
/// account was disabled".
pub const END_SESSIONS_PASSWORD: &str = "UPDATE sessions SET ended_at = now(), \
                                         ended_reason = 'password_changed' \
                                         WHERE user_id = $1 AND ended_at IS NULL";

/// Guarded by `is_active` so a repeat call cannot bump `updated_at` or produce
/// a second audit entry for a change that already happened.
pub const DEACTIVATE_USER: &str = "UPDATE users SET is_active = false, updated_at = now() \
                                   WHERE id = $1 AND is_active";

pub const END_USER_SESSIONS: &str = "UPDATE sessions SET ended_at = now(), \
                                     ended_reason = 'account_disabled' \
                                     WHERE user_id = $1 AND ended_at IS NULL";

pub const REVOKE_USER_TOKENS: &str = "UPDATE refresh_tokens SET revoked_at = now() \
                                      WHERE user_id = $1 AND revoked_at IS NULL";
