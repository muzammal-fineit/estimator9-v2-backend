//! The SQL behind [`super::PgRefreshTokenRepository`].

pub const INSERT_TOKEN: &str = "INSERT INTO refresh_tokens \
                                (user_id, token_hash, session_id, expires_at) \
                                VALUES ($1, $2, $3, $4) RETURNING id";

/// No `revoked_at IS NULL` filter on purpose: a revoked token being presented
/// is the reuse signal, and hiding it here would turn an attack into an
/// ordinary failed refresh.
pub const SELECT_BY_HASH: &str = "SELECT id, user_id, session_id, expires_at, revoked_at \
                                  FROM refresh_tokens WHERE token_hash = $1";

/// Guarded by `revoked_at IS NULL` so a re-revocation cannot move the original
/// timestamp — when the token was first revoked is the forensically useful one.
pub const REVOKE_ONE: &str = "UPDATE refresh_tokens SET revoked_at = $2 \
                              WHERE id = $1 AND revoked_at IS NULL";

pub const REVOKE_SESSION: &str = "UPDATE refresh_tokens SET revoked_at = $2 \
                                  WHERE session_id = $1 AND revoked_at IS NULL";

pub const REVOKE_ALL_FOR_USER: &str = "UPDATE refresh_tokens SET revoked_at = $2 \
                                       WHERE user_id = $1 AND revoked_at IS NULL";
