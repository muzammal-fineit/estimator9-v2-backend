# MySQL migrations

Empty. The layout is here so adding MySQL is a matter of filling a folder rather than
restructuring, but **the folder is not the hard part** — see `db/README.md` for what
actually has to change in the application first.

The Postgres files in `../postgres/` are the reference. Translating them means at
minimum: `bigserial` → `BIGINT AUTO_INCREMENT`, `timestamptz` → `TIMESTAMP`, `jsonb` →
`JSON`, `inet` → `VARBINARY(16)`, `ON CONFLICT DO NOTHING` → `INSERT IGNORE`, and
dropping the functional unique index on `lower(email)` in favour of a generated column.
