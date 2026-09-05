CREATE TABLE IF NOT EXISTS file_shares (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    attachment_id       INTEGER NOT NULL REFERENCES attachments(id) ON DELETE CASCADE,
    shared_with_user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    shared_by           INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at          TEXT    NOT NULL DEFAULT (datetime('now')),
    UNIQUE(attachment_id, shared_with_user_id)
);
CREATE INDEX IF NOT EXISTS idx_file_shares_user ON file_shares(shared_with_user_id);
CREATE INDEX IF NOT EXISTS idx_file_shares_attachment ON file_shares(attachment_id);

CREATE TABLE IF NOT EXISTS file_share_links (
    token          TEXT    PRIMARY KEY,
    attachment_id  INTEGER NOT NULL REFERENCES attachments(id) ON DELETE CASCADE,
    created_by     INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at     TEXT,
    max_downloads  INTEGER,
    download_count INTEGER NOT NULL DEFAULT 0,
    created_at     TEXT    NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_file_share_links_attachment ON file_share_links(attachment_id);
