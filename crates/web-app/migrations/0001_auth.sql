-- Authentication foundation: local accounts, server-side sessions, invites.
-- Stable local user ids are the attribution key for future revision history.

CREATE TABLE users (
    id TEXT PRIMARY KEY NOT NULL,
    provider TEXT NOT NULL,
    provider_subject TEXT NOT NULL,
    display_name TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('viewer', 'editor', 'administrator')),
    is_disabled INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE (provider, provider_subject)
);

CREATE TABLE sessions (
    token_hash TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    revoked_at INTEGER
);

CREATE INDEX idx_sessions_user_id ON sessions (user_id);

CREATE TABLE invites (
    id TEXT PRIMARY KEY NOT NULL,
    provider TEXT NOT NULL,
    provider_subject TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('viewer', 'editor', 'administrator')),
    note TEXT,
    created_at INTEGER NOT NULL,
    consumed_at INTEGER,
    consumed_by TEXT REFERENCES users (id)
);

CREATE UNIQUE INDEX idx_invites_pending ON invites (provider, provider_subject)
WHERE consumed_at IS NULL;
