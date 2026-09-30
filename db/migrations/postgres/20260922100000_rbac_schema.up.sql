-- RBAC: users hold roles, roles bundle permissions.
--
-- Handlers check *permissions*, never role names, so changing who may do what
-- is a data change rather than a deploy. `superadmin` is the one exception:
-- it is granted no permissions at all and the guard short-circuits on it
-- before any lookup happens.
--
-- Structure only. The roles and permissions themselves are reference data and
-- live in db/seeds/postgres/reference/.

CREATE TABLE IF NOT EXISTS roles (
    id          bigserial   PRIMARY KEY,
    name        text        NOT NULL,
    description text        NOT NULL,
    created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS roles_name_key ON roles (name);

CREATE TABLE IF NOT EXISTS permissions (
    id          bigserial   PRIMARY KEY,
    name        text        NOT NULL,
    description text        NOT NULL,
    created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS permissions_name_key ON permissions (name);

CREATE TABLE IF NOT EXISTS role_permissions (
    role_id       bigint NOT NULL REFERENCES roles (id) ON DELETE CASCADE,
    permission_id bigint NOT NULL REFERENCES permissions (id) ON DELETE CASCADE,
    PRIMARY KEY (role_id, permission_id)
);

-- ON DELETE RESTRICT, not CASCADE: dropping a role that people still hold
-- should fail loudly rather than silently strip their access.
CREATE TABLE IF NOT EXISTS user_roles (
    user_id    bigint      NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    role_id    bigint      NOT NULL REFERENCES roles (id) ON DELETE RESTRICT,
    granted_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, role_id)
);

-- The primary keys already cover the user -> roles -> permissions direction.
-- These cover the reverse ("who holds this role"), which the admin screens need.
CREATE INDEX IF NOT EXISTS user_roles_role_id_idx ON user_roles (role_id);
CREATE INDEX IF NOT EXISTS role_permissions_permission_id_idx
    ON role_permissions (permission_id);
