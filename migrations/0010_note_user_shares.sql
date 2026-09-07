-- Direct note shares: owner shares a personal note with a specific registered user (read-only).
CREATE TABLE IF NOT EXISTS note_shares (
    note_id             INTEGER NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    shared_with_user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    shared_by           INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at          TEXT    NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (note_id, shared_with_user_id)
);
CREATE INDEX IF NOT EXISTS idx_note_shares_user ON note_shares(shared_with_user_id);
CREATE INDEX IF NOT EXISTS idx_note_shares_note ON note_shares(note_id);
