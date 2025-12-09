# Music Minder Architecture

## Overview

Music Minder is a cross-platform desktop music player written in Rust. It manages music collections by scanning, identifying, organizing, enriching, and **playing** audio files with high-fidelity reproduction.

## Core Principles

### 🎵 Audio-First Design

**The primary goal is excellent audio playback.** Everything else (UI, metadata, organization) exists to support the listening experience. This means:

1. **Real-time safety**: The audio pipeline never blocks, allocates, or takes locks in the hot path
2. **Bit-perfect playback**: No unnecessary DSP unless explicitly enabled by the user
3. **Low latency**: Minimize buffer sizes while avoiding underruns
4. **Format support**: Native decoding of MP3, FLAC, OGG, WAV, M4A/AAC
5. **Device flexibility**: WASAPI (Windows), CoreAudio (macOS), ALSA/PulseAudio (Linux)

### Audio Pipeline Architecture

```text
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Decoder   │────▶│  Resampler  │────▶│ Ring Buffer │────▶│ CPAL Output │
│  (Symphonia)│     │  (Rubato)   │     │   (rtrb)    │     │  (WASAPI)   │
└─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘
      │                                        │                   │
      │ Decode Thread                          │ Lock-Free         │ RT Thread
      │ (normal priority)                      │ (no alloc)        │ (high priority)
      ▼                                        ▼                   ▼
┌─────────────┐                          ┌─────────────┐     ┌─────────────┐
│     FFT     │                          │   Atomics   │◀────│   Volume    │
│ Visualizer  │                          │ (position)  │     │  Control    │
└─────────────┘                          └─────────────┘     └─────────────┘
```

**Key guarantees:**

- No `Mutex`/`RwLock` in the audio callback — atomics only
- No heap allocation in the audio callback — ring buffer pre-allocated
- No blocking I/O in the audio callback — decoder runs in separate thread

## Tech Stack

- **Language**: Rust (2024 edition)
- **GUI Framework**: [Iced 0.13](https://github.com/iced-rs/iced) (Cross-platform, type-safe, Elm-inspired)
- **Audio Playback**: `cpal` (platform audio), `symphonia` (decoding), `rubato` (resampling)
- **Database**: SQLite via `sqlx` for library indexing
- **Audio Metadata**: `lofty` for reading/writing tags
- **Async Runtime**: `tokio`

## Core Modules

### Audio Pipeline (Critical Path)

1. **Player**: Orchestrates playback, queue management, state
2. **Decoder**: Symphonia-based format decoding (runs in dedicated thread)
3. **Resampler**: Rubato-based sample rate conversion (when device rate ≠ file rate)
4. **Audio Output**: cpal stream management, ring buffer consumer, volume control
5. **Visualization**: FFT-based spectrum analyzer (decoupled from audio path)

### Library Management

1. **Scanner**: Recursive directory walker to find audio files
2. **Metadata**: Abstraction layer for reading/writing ID3, Vorbis, FLAC tags via lofty
3. **Database**: SQLite schema for tracks, albums, artists, health records
4. **Organizer**: Rule-based engine for moving/renaming files

### Enrichment

1. **Fingerprint**: fpcalc integration for audio fingerprinting
2. **AcoustID**: Audio fingerprint lookup API
3. **MusicBrainz**: Detailed metadata lookup by recording ID
4. **Cover Art**: Cover Art Archive integration + local cache

### User Interface

1. **UI State**: Application state management (Elm architecture)
2. **Views**: Library, Now Playing, Settings panes
3. **Canvas**: Custom visualization rendering

## Data Flow

1. **Scan**: User selects a folder -> Scanner walks it -> Metadata extracted -> Stored in DB.
2. **View**: UI queries DB -> Displays Library.
3. **Action**: User selects "Organize" -> Organizer reads DB/Files -> Moves Files -> Updates DB.

## Directory Structure

```text
music-minder/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point, CLI + GUI dispatch
│   ├── error.rs             # Error types
│   ├── cli/                 # Command-line interface
│   ├── db/                  # SQLite database layer
│   ├── model/               # Database entities
│   ├── scanner/             # File system scanning
│   ├── metadata/            # Tag reading/writing (lofty)
│   ├── organizer/           # File organization engine
│   ├── health/              # File health tracking
│   ├── enrichment/          # External API integrations
│   │   ├── acoustid/        # Audio fingerprint lookup
│   │   ├── musicbrainz/     # Metadata lookup
│   │   └── coverart/        # Cover art fetching
│   ├── cover/               # Cover art resolution & caching
│   ├── player/              # 🔊 Audio pipeline (critical path)
│   │   ├── audio.rs         # cpal output, ring buffer
│   │   ├── decoder.rs       # Symphonia decoding thread
│   │   ├── resampler.rs     # Rubato sample rate conversion
│   │   ├── queue.rs         # Playback queue management
│   │   ├── state.rs         # Atomics for lock-free control
│   │   └── visualization.rs # FFT spectrum analyzer
│   ├── diagnostics/         # System audio readiness checks
│   └── ui/                  # Iced GUI
│       ├── state.rs         # Application state
│       ├── messages.rs      # Elm-style messages
│       ├── update.rs        # State transitions
│       ├── canvas.rs        # Custom rendering
│       └── views/           # UI panes
├── migrations/              # SQLite schema migrations
├── assets/                  # Icons, fonts
└── docs/                    # Project documentation
```

## Player Control Flow

The player has multiple entry points for the same actions (UI buttons, keyboard, OS media keys). To avoid duplicate logic and inconsistent behavior, we use a **single canonical handler** pattern.

### Control Flow Diagram

```text
┌─────────────────────────────────────────────────────────────────┐
│                        Entry Points                             │
├─────────────────────────────────────────────────────────────────┤
│ UI Button        → Message::PlayerPlay/Pause/Next/etc           │
│ OS Media Keys    → MediaControlPoll → MediaControlCommand       │
│ Keyboard Shortcut→ Message::PlayerToggle/etc                    │
└──────────────────────────┬──────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                   mod.rs - Message Router                       │
│  • MediaControlPoll: polls OS, emits MediaControlCommand        │
│  • Routes all player messages to handle_player()                │
└──────────────────────────┬──────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│              handle_player() - Single Handler                   │
│  • MediaControlCommand converts to equivalent Player* action    │
│  • Each action implemented ONCE via helper functions            │
└──────────────────────────┬──────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│              Internal Helpers (private functions)               │
│  do_play()   - player.play() + state sync + SMTC update         │
│  do_pause()  - player.pause() + state sync + SMTC update        │
│  do_next()   - player.skip_forward() + state + metadata         │
│  do_seek()   - player.seek() + state sync                       │
│  etc.                                                           │
└─────────────────────────────────────────────────────────────────┘
```

### Key Principles

1. **One implementation per action**: Play/pause/next/etc have a single code path
2. **Consistent error handling**: All paths report errors to `status_message`
3. **Consistent state sync**: All paths update `player_state` and OS media controls
4. **Metadata updates on track change**: Next/Previous/PlayTrack update SMTC metadata

### Message Types

| Message | Source | Purpose |
|---------|--------|---------|
| `PlayerPlay/Pause/etc` | UI buttons | Direct user interaction |
| `MediaControlCommand` | OS media keys (via poll) | External control |
| `PlayerTick` | Timer subscription | Periodic state sync |
| `PlayerVisualizationTick` | Fast timer | FFT data for visualizer |

### OS Media Controls (SMTC/MPRIS)

The OS media controls run on a dedicated thread (`media_controls.rs`) and communicate via channels:

- **Outbound** (to OS): metadata, playback state, position
- **Inbound** (from OS): play/pause/next/prev/seek commands

Polling happens in `mod.rs` via `MediaControlPoll` subscription (50ms interval). Commands are converted to `MediaControlCommand` messages and routed through the normal handler.

## Design Decisions

### Why Rust?

- **Memory safety** without garbage collection pauses
- **Fearless concurrency** for audio + UI + network threads
- **Zero-cost abstractions** for real-time performance
- **Excellent ecosystem** for audio (cpal, symphonia, rubato)

### Why Iced?

- Pure Rust, no C++ dependencies
- Elm architecture fits audio player state management
- Cross-platform (Windows, macOS, Linux)
- Good performance with GPU-accelerated rendering

### Why SQLite?

- Embedded, no server needed
- Fast for local queries (10k+ tracks)
- ACID transactions for safe library updates
- Easy to backup (single file)
