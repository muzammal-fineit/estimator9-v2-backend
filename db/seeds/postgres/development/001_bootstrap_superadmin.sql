-- DEVELOPMENT ONLY. Never run this set against production.
--
-- Grants superadmin to the lowest-numbered existing account so there is
-- somebody who can grant roles to everyone else. In production the first
-- superadmin is created deliberately by an operator, not by whoever happens
-- to hold id 1.

INSERT INTO user_roles (user_id, role_id)
SELECT u.id, r.id
FROM users u
JOIN roles r ON r.name = 'superadmin'
WHERE u.id = (SELECT min(id) FROM users)
ON CONFLICT DO NOTHING;
