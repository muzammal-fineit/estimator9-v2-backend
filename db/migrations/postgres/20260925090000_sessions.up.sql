-- Sessions: one row per successful login, tracking its life and how it ended.
--
-- This splits two things that were previously tangled in `audit_log`. A
-- session is *state* — currently alive, last seen at, ended for this reason —
-- and it changes. An audit entry is an *event*: it happened, and it is never
-- revised. Keeping session state out of the append-only trail means the trail
-- stays append-only, and "which sessions are open right now" becomes a query
-- against a small table rather than a reconstruction from a log.
--
-- `audit_log.session_id` joins them, so "everything that happened during this
-- login" is one query.

CREATE TABLE IF NOT EXISTS sessions (
    id           bigserial   PRIMARY KEY,
    user_id      bigint      NOT NULL REFERENCES users (id) ON DELETE CASCADE,

    started_at   timestamptz NOT NULL DEFAULT now(),

    -- Advanced on every refresh. Not on every request: that would be a write
    -- per call for a figure nobody needs to the second.
    last_seen_at timestamptz NOT NULL DEFAULT now(),

    ended_at     timestamptz,

    -- Null while alive. One of: logged_out, expired, token_reused,
    -- password_changed, account_disabled, revoked_by_admin.
    -- Constrained in the domain rather than by a CHECK, so adding a reason is
    -- a code change reviewed with its behaviour rather than a silent string.
    ended_reason text,

    -- Where the session began. Later requests may come from elsewhere; that is
    -- visible in the audit rows, and a change mid-session is worth noticing.
    ip_address   inet,
    user_agent   text
);

-- "Which sessions does this user have open" — the admin screen, and the
-- revoke-everything paths.
CREATE INDEX IF NOT EXISTS sessions_user_active_idx
    ON sessions (user_id) WHERE ended_at IS NULL;


-- Refresh tokens belong to a session ------------------------------------------
--
-- `family_id` was already a session in all but name: the chain of rotated
-- tokens from one login. Replacing it with a real foreign key removes the
-- second, parallel notion of a session.

DELETE FROM refresh_tokens;
-- ^ Outstanding tokens cannot be mapped onto sessions that were never
-- recorded. Everyone logs in again once. Safe here because refresh tokens have
-- existed for a day and this has not been released; on a live installation
-- this would be an add-backfill-switch sequence instead.

ALTER TABLE refresh_tokens
    ADD COLUMN IF NOT EXISTS session_id bigint NOT NULL
    REFERENCES sessions (id) ON DELETE CASCADE;

ALTER TABLE refresh_tokens DROP COLUMN IF EXISTS family_id;

CREATE INDEX IF NOT EXISTS refresh_tokens_session_idx
    ON refresh_tokens (session_id);


-- Audit entries name their session --------------------------------------------
--
-- Nullable: a failed login has no session, and that is the common case for the
-- rows a security rule fires on.

ALTER TABLE audit_log
    ADD COLUMN IF NOT EXISTS session_id bigint
    REFERENCES sessions (id) ON DELETE SET NULL;
