-- Public share links for personal notes (read-only, no login required).
CREATE TABLE IF NOT EXISTS note_share_links (
    token       TEXT    PRIMARY KEY,
    note_id     INTEGER NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    created_by  INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at  TEXT,
    created_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_note_share_links_note ON note_share_links(note_id);
