CREATE TABLE IF NOT EXISTS attachments (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    owner_id        INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    scope           TEXT    NOT NULL CHECK (scope IN ('note','shared_page')),
    scope_id        INTEGER NOT NULL,
    filename        TEXT    NOT NULL,
    content_type    TEXT    NOT NULL,
    byte_size       INTEGER NOT NULL,
    stored_name     TEXT    NOT NULL UNIQUE,
    created_at      TEXT    NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_attachments_scope ON attachments(scope, scope_id);
