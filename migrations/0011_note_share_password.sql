-- Add optional password protection to public note share links.
ALTER TABLE note_share_links ADD COLUMN password_hash TEXT;
