# Music Minder Architecture

## Overview
Music Minder is a Windows desktop application written in Rust. It manages music collections by scanning, identifying, organizing, and enriching audio files.

## Tech Stack
- **Language**: Rust (2021 edition)
- **GUI Framework**: [Iced](https://github.com/iced-rs/iced) (Cross-platform, type-safe, Elm-inspired)
- **Database**: SQLite (via `sqlx` or `rusqlite`) for library indexing.
- **Audio Metadata**: `lofty` or `symphonia` for reading/writing tags.
- **Async Runtime**: `tokio`

## Core Modules
1.  **Domain**: Core data structures (Track, Album, Artist).
2.  **Scanner**: Recursive directory walker to find audio files.
3.  **Indexer**: Manages the SQLite database, syncing file system state with the DB.
4.  **Metadata**: Abstraction layer for reading/writing ID3, Vorbis, FLAC tags.
5.  **Organizer**: Rule-based engine for moving/renaming files (e.g., `{Artist}/{Album}/{Track} - {Title}.mp3`).
6.  **Enricher**: HTTP client for external APIs (MusicBrainz, AcoustID, Spotify).
7.  **UI**: The Iced application state and view logic.

## Data Flow
1.  **Scan**: User selects a folder -> Scanner walks it -> Metadata extracted -> Stored in DB.
2.  **View**: UI queries DB -> Displays Library.
3.  **Action**: User selects "Organize" -> Organizer reads DB/Files -> Moves Files -> Updates DB.

## Directory Structure
```
music-minder/
├── Cargo.toml
├── src/
│   ├── main.rs          # Entry point, UI setup
│   ├── model/           # Database entities
│   ├── scanner/         # File system scanning
│   ├── metadata/        # Tag handling
│   ├── db/              # Database interactions
│   └── ui/              # Iced components and pages
└── docs/                # Project documentation
```
