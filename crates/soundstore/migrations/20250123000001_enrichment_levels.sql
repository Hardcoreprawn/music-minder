-- Track enrichment level for graceful degradation
-- Records what level of enrichment has been achieved for each track

-- Add enrichment_level column to tracks table
-- Values: 'minimal', 'basic', 'enhanced', 'complete'
-- minimal  = File metadata only
-- basic    = AcoustID fingerprint match
-- enhanced = Basic + MusicBrainz enrichment
-- complete = Enhanced + cover art
ALTER TABLE tracks ADD COLUMN enrichment_level TEXT DEFAULT 'minimal';

-- Add cover_art_available flag (we don't store the art itself in DB)
ALTER TABLE tracks ADD COLUMN cover_art_available INTEGER DEFAULT 0;

-- Index for finding tracks by enrichment level (for batch enrichment)
CREATE INDEX IF NOT EXISTS idx_tracks_enrichment_level ON tracks(enrichment_level);

-- View for tracks that could benefit from enrichment
CREATE VIEW IF NOT EXISTS tracks_enrichable AS
SELECT 
    t.*,
    a.name as artist_name,
    al.title as album_title
FROM tracks t
LEFT JOIN artists a ON t.artist_id = a.id
LEFT JOIN albums al ON t.album_id = al.id
WHERE t.enrichment_level != 'complete'
ORDER BY 
    CASE t.enrichment_level
        WHEN 'minimal' THEN 0
        WHEN 'basic' THEN 1
        WHEN 'enhanced' THEN 2
        WHEN 'complete' THEN 3
        ELSE 0
    END ASC,
    t.title;
