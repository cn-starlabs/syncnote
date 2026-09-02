ALTER TABLE users ADD COLUMN is_admin INTEGER NOT NULL DEFAULT 0;

CREATE TABLE IF NOT EXISTS invite_codes (
    code        TEXT    PRIMARY KEY,
    created_by  INTEGER REFERENCES users(id) ON DELETE SET NULL,
    uses_left   INTEGER NOT NULL DEFAULT 1,
    expires_at  TEXT,
    note        TEXT,
    created_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);
