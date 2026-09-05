-- SQLite can't ALTER a CHECK constraint or drop NOT NULL in place, so this
-- rebuilds the table: adds 'library' as a valid scope (a personal file not
-- tied to any specific note/page at upload time — usable later across notes,
-- like a small personal drive) and makes scope_id nullable to match.
CREATE TABLE attachments_new (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    owner_id        INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    scope           TEXT    NOT NULL CHECK (scope IN ('note','shared_page','library')),
    scope_id        INTEGER,
    filename        TEXT    NOT NULL,
    content_type    TEXT    NOT NULL,
    byte_size       INTEGER NOT NULL,
    stored_name     TEXT    NOT NULL UNIQUE,
    created_at      TEXT    NOT NULL DEFAULT (datetime('now'))
);

INSERT INTO attachments_new (id, owner_id, scope, scope_id, filename, content_type, byte_size, stored_name, created_at)
    SELECT id, owner_id, scope, scope_id, filename, content_type, byte_size, stored_name, created_at FROM attachments;

DROP TABLE attachments;
ALTER TABLE attachments_new RENAME TO attachments;

CREATE INDEX IF NOT EXISTS idx_attachments_scope ON attachments(scope, scope_id);
CREATE INDEX IF NOT EXISTS idx_attachments_owner ON attachments(owner_id);
