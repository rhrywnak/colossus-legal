-- Track users who have logged into the application.
-- Populated automatically by middleware on each authenticated request.
-- Used to populate reviewer dropdown and other user selection UIs.
CREATE TABLE IF NOT EXISTS known_users (
    username TEXT PRIMARY KEY,
    display_name TEXT NOT NULL DEFAULT '',
    email TEXT NOT NULL DEFAULT '',
    first_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Add reviewer assignment to documents table
ALTER TABLE documents ADD COLUMN IF NOT EXISTS assigned_reviewer TEXT;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS assigned_at TIMESTAMPTZ;
