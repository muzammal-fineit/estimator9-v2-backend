//! The SQL behind [`super::PgSessionRepository`].

pub const INSERT_SESSION: &str = "INSERT INTO sessions (user_id, ip_address, user_agent) \
                                  VALUES ($1, $2, $3) \
                                  RETURNING id, user_id, started_at, last_seen_at, \
                                            ended_at, ended_reason";

pub const SELECT_BY_ID: &str = "SELECT id, user_id, started_at, last_seen_at, \
                                       ended_at, ended_reason \
                                FROM sessions WHERE id = $1";

pub const TOUCH: &str = "UPDATE sessions SET last_seen_at = $2 \
                         WHERE id = $1 AND ended_at IS NULL";

/// Guarded by `ended_at IS NULL` so the first reason survives. If a session is
/// ended by token reuse and then someone calls logout, the row should keep
/// saying `token_reused` — that is the fact worth having.
pub const END_ONE: &str = "UPDATE sessions SET ended_at = $2, ended_reason = $3 \
                           WHERE id = $1 AND ended_at IS NULL";

pub const END_ALL_FOR_USER: &str = "UPDATE sessions SET ended_at = $2, ended_reason = $3 \
                                    WHERE user_id = $1 AND ended_at IS NULL";
