-- Alternative matches storage
-- When fingerprinting finds multiple possible matches, store them here
-- so users can review and pick the best one

-- Stores candidate matches for a track from fingerprint identification
CREATE TABLE IF NOT EXISTS track_matches (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    track_id INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    
    -- Match source and confidence
    source TEXT NOT NULL DEFAULT 'acoustid',  -- 'acoustid', 'musicbrainz', 'manual'
    confidence REAL NOT NULL,  -- 0.0-1.0
    
    -- Recording information from fingerprint
    recording_id TEXT,  -- MusicBrainz recording ID
    recording_title TEXT NOT NULL,
    recording_artist TEXT,
    recording_duration_ms INTEGER,
    
    -- Comparison with current metadata
    title_similarity REAL,  -- 0.0-1.0 how close to current title
    artist_similarity REAL,  -- 0.0-1.0 how close to current artist
    
    -- User selection
    is_selected BOOLEAN DEFAULT FALSE,  -- User picked this match
    is_rejected BOOLEAN DEFAULT FALSE,  -- User explicitly rejected this
    
    -- Timestamps
    discovered_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    reviewed_at DATETIME,  -- When user reviewed this match
    
    UNIQUE(track_id, recording_id)
);

-- Stores release (album) options for a recording
-- A single recording can appear on many albums (compilations, reissues, etc.)
CREATE TABLE IF NOT EXISTS match_releases (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    match_id INTEGER NOT NULL REFERENCES track_matches(id) ON DELETE CASCADE,
    
    -- Release information
    release_id TEXT NOT NULL,  -- MusicBrainz release ID
    release_title TEXT NOT NULL,
    release_artist TEXT,
    release_year INTEGER,
    release_type TEXT,  -- 'album', 'single', 'compilation', 'ep', 'live', etc.
    
    -- Track position
    track_number INTEGER,
    disc_number INTEGER,
    total_tracks INTEGER,
    
    -- Quality indicators
    is_original_release BOOLEAN DEFAULT FALSE,  -- First release of this recording
    is_compilation BOOLEAN DEFAULT FALSE,
    country TEXT,  -- Release country code
    
    -- Cover art
    cover_art_url TEXT,
    
    -- User preference
    is_preferred BOOLEAN DEFAULT FALSE,  -- User prefers this release
    
    UNIQUE(match_id, release_id)
);

-- Index for finding matches needing review
CREATE INDEX IF NOT EXISTS idx_track_matches_unreviewed 
ON track_matches(track_id) 
WHERE is_selected = FALSE AND is_rejected = FALSE;

-- Index for finding tracks with multiple good matches
CREATE INDEX IF NOT EXISTS idx_track_matches_confidence
ON track_matches(track_id, confidence DESC);

-- Index for finding compilation releases
CREATE INDEX IF NOT EXISTS idx_match_releases_compilation
ON match_releases(match_id)
WHERE is_compilation = TRUE;
