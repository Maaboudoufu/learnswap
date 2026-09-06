-- Every column is TEXT so the same DDL runs unchanged on SQLite and Postgres.
-- ponytail: skills live in comma-separated columns instead of a join table.
-- Search is a LIKE scan, which is fine at class-project scale; normalise into
-- skills/user_skills tables if the member list ever outgrows one page.
CREATE TABLE IF NOT EXISTS users (
    id            TEXT PRIMARY KEY,
    email         TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    name          TEXT NOT NULL,
    headline      TEXT NOT NULL DEFAULT '',
    bio           TEXT NOT NULL DEFAULT '',
    teaching      TEXT NOT NULL DEFAULT '',
    learning      TEXT NOT NULL DEFAULT ''
);
