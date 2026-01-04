-- Add mtime column for incremental scanning
-- Stores the file's last modified time as Unix timestamp

ALTER TABLE tracks ADD COLUMN mtime INTEGER;

-- Index for efficient mtime queries during incremental scans
CREATE INDEX IF NOT EXISTS idx_tracks_mtime ON tracks(mtime);
