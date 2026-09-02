CREATE TABLE IF NOT EXISTS passkeys (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id         INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    label           TEXT,
    passkey_json    TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    last_used_at    TEXT
);
CREATE INDEX IF NOT EXISTS idx_passkeys_user ON passkeys(user_id);
