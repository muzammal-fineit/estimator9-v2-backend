-- One-time tokens for "I forgot my password".
--
-- Stored as a hash for the same reason refresh tokens are: a dump of this
-- table must not hand anyone a way into an account. SHA-256 rather than
-- Argon2, again for the same reason — these are 256 bits of randomness with
-- nothing to guess, so a slow hash would only add latency.

CREATE TABLE IF NOT EXISTS password_reset_tokens (
    id         bigserial   PRIMARY KEY,
    user_id    bigint      NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    token_hash text        NOT NULL,

    -- Short: a reset link sitting in a mailbox is a standing key to the
    -- account, so it should stop being one quickly.
    expires_at timestamptz NOT NULL,

    -- Set when redeemed. The row is kept rather than deleted so a replay is
    -- distinguishable from a token that never existed.
    used_at    timestamptz,

    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS password_reset_tokens_hash_key
    ON password_reset_tokens (token_hash);

-- Superseding a user's outstanding tokens when they ask again.
CREATE INDEX IF NOT EXISTS password_reset_tokens_user_idx
    ON password_reset_tokens (user_id) WHERE used_at IS NULL;
