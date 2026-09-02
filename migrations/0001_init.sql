CREATE TABLE IF NOT EXISTS users (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    email           TEXT    NOT NULL UNIQUE,
    password_hash   TEXT    NOT NULL,
    display_name    TEXT,
    created_at      TEXT    NOT NULL DEFAULT (datetime('now')),
    last_login_at   TEXT
);

-- Personal notes: only the owner ever reads/writes these.
CREATE TABLE IF NOT EXISTS notes (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    owner_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title       TEXT    NOT NULL DEFAULT 'Untitled',
    body        TEXT    NOT NULL DEFAULT '',
    created_at  TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_notes_owner ON notes(owner_id);

-- Shared pages: multiple users can view/edit depending on shared_page_members.role.
-- `version` is bumped on every save and used to detect a concurrent-edit race
-- over the WebSocket sync channel (see DEVPLAN.md).
CREATE TABLE IF NOT EXISTS shared_pages (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    owner_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title       TEXT    NOT NULL DEFAULT 'Untitled',
    body        TEXT    NOT NULL DEFAULT '',
    version     INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS shared_page_members (
    page_id     INTEGER NOT NULL REFERENCES shared_pages(id) ON DELETE CASCADE,
    user_id     INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role        TEXT    NOT NULL CHECK (role IN ('owner','editor','viewer')),
    joined_at   TEXT    NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (page_id, user_id)
);
CREATE INDEX IF NOT EXISTS idx_members_user ON shared_page_members(user_id);

CREATE TABLE IF NOT EXISTS shared_page_invites (
    token       TEXT    PRIMARY KEY,
    page_id     INTEGER NOT NULL REFERENCES shared_pages(id) ON DELETE CASCADE,
    role        TEXT    NOT NULL CHECK (role IN ('editor','viewer')),
    created_by  INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at  TEXT,
    created_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_invites_page ON shared_page_invites(page_id);
