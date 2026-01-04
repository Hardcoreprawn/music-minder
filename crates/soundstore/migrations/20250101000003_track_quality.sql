-- Track quality assessment for metadata nurturing
-- Stores quality scores and flags for background enrichment suggestions

-- Add quality columns to tracks table
ALTER TABLE tracks ADD COLUMN quality_score INTEGER DEFAULT NULL;
ALTER TABLE tracks ADD COLUMN quality_flags INTEGER DEFAULT NULL;
ALTER TABLE tracks ADD COLUMN quality_checked_at TEXT DEFAULT NULL;
ALTER TABLE tracks ADD COLUMN acoustid_confidence REAL DEFAULT NULL;
ALTER TABLE tracks ADD COLUMN musicbrainz_recording_id TEXT DEFAULT NULL;

-- Index for finding tracks that need attention
CREATE INDEX IF NOT EXISTS idx_tracks_quality_score ON tracks(quality_score);
CREATE INDEX IF NOT EXISTS idx_tracks_quality_flags ON tracks(quality_flags);
CREATE INDEX IF NOT EXISTS idx_tracks_quality_checked ON tracks(quality_checked_at);

-- View for tracks needing enrichment (score < 70 or never checked)
CREATE VIEW IF NOT EXISTS tracks_needing_attention AS
SELECT 
    t.*,
    a.name as artist_name,
    al.title as album_title
FROM tracks t
LEFT JOIN artists a ON t.artist_id = a.id
LEFT JOIN albums al ON t.album_id = al.id
WHERE t.quality_score IS NULL 
   OR t.quality_score < 70
   OR t.quality_flags & 512 != 0  -- NEVER_CHECKED flag (1 << 9)
ORDER BY 
    CASE WHEN t.quality_score IS NULL THEN 0 ELSE t.quality_score END ASC,
    t.title;
