-- Append-only audit trail. Every row is written by the application, never by a
-- trigger, so each entry carries the business meaning of what happened
-- ("user.roles.assigned") rather than the HTTP verb that caused it.
--
-- Scope worth knowing: this records what goes *through the API*. Someone with
-- direct psql access can change data without leaving a row here. That is a
-- separate control — see the hardening step in DEPLOYMENT.md.

CREATE TABLE IF NOT EXISTS audit_log (
    id             bigserial   PRIMARY KEY,

    -- Null for unauthenticated events: a failed login has no actor yet.
    -- SET NULL rather than CASCADE — deleting a user must never delete the
    -- record of what they did.
    actor_id       bigint      REFERENCES users (id) ON DELETE SET NULL,

    -- Denormalised on purpose: survives the actor row changing or going away.
    actor_email    text,

    -- True when the action was permitted by the superadmin bypass rather than
    -- by a granted permission. The column an auditor will filter on first.
    via_superadmin boolean     NOT NULL DEFAULT false,

    action         text        NOT NULL,
    resource_type  text,
    resource_id    text,

    -- Flat string map. Never contains passwords, hashes or tokens — enforced
    -- in the domain, with a test that asserts it.
    metadata       jsonb       NOT NULL DEFAULT '{}'::jsonb,

    ip_address     inet,
    user_agent     text,
    created_at     timestamptz NOT NULL DEFAULT now()
);

-- Deliberately no secondary indexes yet. Every index slows the INSERT that sits
-- in the same transaction as the write being audited, and there is no query
-- pattern to index for until something reads this table. Add (actor_id) and
-- (created_at DESC) when the audit screen exists, not before.
