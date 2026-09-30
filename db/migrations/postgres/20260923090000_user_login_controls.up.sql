-- Account-level brute-force defence.
--
-- Per-IP rate limiting (phase 3) does not cover an attack spread across many
-- addresses against one account. These two columns are that second bucket:
-- the counter is keyed by the account, wherever the attempt came from.

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS failed_login_attempts integer NOT NULL DEFAULT 0;

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS locked_until timestamptz;
