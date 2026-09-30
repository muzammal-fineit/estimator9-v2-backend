-- Reference data: the authorization vocabulary the application code depends on.
--
-- This is NOT optional sample data. `RoleName::SUPERADMIN` and the permission
-- names checked in handlers are meaningless without these rows — a database
-- with the schema but not this file has a working API and a broken
-- authorization system. Deployment runs it on every release.
--
-- Idempotent, so re-running is always safe.

INSERT INTO roles (name, description) VALUES
    ('superadmin', 'Vendor account. Enables and disables software features on a deployed installation, so that control does not rest with the client. Exempt from roles and permissions, never from audit. Not grantable through the roles endpoint.'),
    ('admin',      'Manages user accounts and role assignments.'),
    ('approver',   'Approves provision runs.'),
    ('analyst',    'Fits MEV models and reviews results.'),
    ('viewer',     'Read-only access.')
-- Descriptions are corrected in place. DO NOTHING would leave an installation
-- that seeded an earlier release describing these roles wrongly forever, and
-- the description is what an auditor reads to understand the model.
ON CONFLICT (name) DO UPDATE SET description = EXCLUDED.description;

INSERT INTO permissions (name, description) VALUES
    ('users.read',        'View user accounts'),
    ('users.manage',      'Create, update and deactivate users; assign roles'),
    ('roles.read',        'View roles and permissions'),
    ('audit.read',        'Read the audit log'),
    ('mev.read',          'View MEV series and fitted models'),
    ('mev.fit',           'Run MEV regression fits'),
    ('provision.read',    'View provision runs'),
    ('provision.approve', 'Approve provision runs')
ON CONFLICT (name) DO UPDATE SET description = EXCLUDED.description;

-- Note the separation of duties: `admin` manages people but cannot approve a
-- provision run, and `approver` can approve but cannot grant itself to anyone.
-- Collapsing those two is the change an auditor will ask you to justify.
--
-- `superadmin` appears nowhere below, by design.
--
-- It is not "an admin with everything" — it is the vendor's account on a
-- client installation, existing to toggle software features the client should
-- not control. Roles and permissions are *customer* policy, so it sits outside
-- them and the guard short-circuits before any lookup. It is emphatically not
-- outside the audit trail: every action it takes is recorded with
-- `via_superadmin = true`, and a client administrator holding `audit.read` can
-- see all of them. They cannot grant, revoke or disable the role.
--
-- Granting it a permission row here would be a bug, not a convenience.
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM (VALUES
    ('admin',    'users.read'),
    ('admin',    'users.manage'),
    ('admin',    'roles.read'),
    ('admin',    'audit.read'),
    ('admin',    'mev.read'),
    ('admin',    'provision.read'),

    ('approver', 'users.read'),
    ('approver', 'mev.read'),
    ('approver', 'provision.read'),
    ('approver', 'provision.approve'),

    ('analyst',  'mev.read'),
    ('analyst',  'mev.fit'),
    ('analyst',  'provision.read'),

    ('viewer',   'mev.read'),
    ('viewer',   'provision.read')
) AS m (role_name, permission_name)
JOIN roles       r ON r.name = m.role_name
JOIN permissions p ON p.name = m.permission_name
ON CONFLICT DO NOTHING;
