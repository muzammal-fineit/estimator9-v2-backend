ALTER TABLE audit_log DROP COLUMN IF EXISTS session_id;

DELETE FROM refresh_tokens;
ALTER TABLE refresh_tokens DROP COLUMN IF EXISTS session_id;
ALTER TABLE refresh_tokens ADD COLUMN IF NOT EXISTS family_id text NOT NULL;

DROP TABLE IF EXISTS sessions;
