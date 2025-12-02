CREATE TABLE IF NOT EXISTS artists (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS albums (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    artist_id INTEGER,
    year INTEGER,
    FOREIGN KEY(artist_id) REFERENCES artists(id)
);

CREATE TABLE IF NOT EXISTS tracks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    artist_id INTEGER,
    album_id INTEGER,
    path TEXT NOT NULL UNIQUE,
    duration INTEGER, -- in seconds
    track_number INTEGER,
    FOREIGN KEY(artist_id) REFERENCES artists(id),
    FOREIGN KEY(album_id) REFERENCES albums(id)
);
