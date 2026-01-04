-- File health tracking for detecting corrupt/problematic audio files
CREATE TABLE IF NOT EXISTS file_health (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL UNIQUE,
    
    -- Health status: ok, error, no_match, low_confidence
    status TEXT NOT NULL DEFAULT 'unknown',
    
    -- For errors: decode_error, empty_fingerprint, io_error, etc.
    error_type TEXT,
    error_message TEXT,
    
    -- Identification results
    acoustid_fingerprint TEXT,
    acoustid_confidence REAL,
    musicbrainz_id TEXT,
    
    -- File integrity
    file_size INTEGER,
    file_hash TEXT,  -- SHA256 of first 1MB + last 1MB for large files
    
    -- Timestamps
    last_checked TEXT NOT NULL,  -- ISO8601
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_file_health_status ON file_health(status);
CREATE INDEX IF NOT EXISTS idx_file_health_path ON file_health(path);
