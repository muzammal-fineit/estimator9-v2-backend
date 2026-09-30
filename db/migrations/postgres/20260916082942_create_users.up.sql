-- Smoke-test migration: verifies the sqlx migration runner end to end.
-- Not the real auth schema — roles and permissions arrive with the RBAC slice.

CREATE TABLE IF NOT EXISTS users (
    id            bigserial   PRIMARY KEY,
    email         text        NOT NULL,
    name          text        NOT NULL,
    password_hash text        NOT NULL,
    is_active     boolean     NOT NULL DEFAULT true,
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now()
);

-- Case-insensitive uniqueness: Bank.User@x.com and bank.user@x.com are one account.
CREATE UNIQUE INDEX IF NOT EXISTS users_email_lower_key ON users (lower(email));
