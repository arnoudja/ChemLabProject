-- ChemLab v0.1: users + server-side sessions (login from day one).
-- Multiplayer hooks (lab_id ownership, command log) deferred; nullable columns reserved.

CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY NOT NULL,
    email TEXT NOT NULL COLLATE NOCASE UNIQUE,
    display_name TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);

-- Reserved for future lab saves / multiplayer (unused in 0.1).
CREATE TABLE IF NOT EXISTS labs (
    id TEXT PRIMARY KEY NOT NULL,
    owner_user_id TEXT REFERENCES users(id) ON DELETE SET NULL,
    name TEXT NOT NULL DEFAULT 'Lab',
    state_blob BLOB,
    version INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
