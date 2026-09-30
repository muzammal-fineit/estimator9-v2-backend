-- Refresh tokens, stored hashed and rotated on every use.
--
-- `family_id` groups a rotation chain. When a token is exchanged it is revoked
-- and its successor joins the same family, so presenting a already-revoked
-- token means one of two things: a stolen token being replayed, or the
-- legitimate holder replaying after the thief got there first. Either way the
-- safe response is to revoke the whole family and make everyone log in again.

CREATE TABLE IF NOT EXISTS refresh_tokens (
    id         bigserial   PRIMARY KEY,
    user_id    bigint      NOT NULL REFERENCES users (id) ON DELETE CASCADE,

    -- SHA-256 of the secret, hex encoded. The secret itself is shown once, to
    -- the client, and never stored: a dump of this table yields no usable
    -- session.
    --
    -- SHA-256 rather than Argon2 on purpose. Argon2's cost defends against
    -- dictionary attacks on human-chosen passwords; these are 256 bits of
    -- randomness with nothing to guess, so a slow hash would only add latency
    -- to every refresh.
    token_hash text        NOT NULL,

    family_id  text        NOT NULL,

    expires_at timestamptz NOT NULL,
    revoked_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now()
);

-- The lookup on every refresh is by hash, and it must be unique.
CREATE UNIQUE INDEX IF NOT EXISTS refresh_tokens_hash_key
    ON refresh_tokens (token_hash);

-- Revoking a whole family on reuse, and listing a user's sessions on logout.
CREATE INDEX IF NOT EXISTS refresh_tokens_family_idx
    ON refresh_tokens (family_id);
CREATE INDEX IF NOT EXISTS refresh_tokens_user_idx
    ON refresh_tokens (user_id);
