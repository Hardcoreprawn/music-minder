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

### 🔧 CLI-First Development

**Every feature is built CLI-first, then wrapped with GUI.** This enables:

- **AI-assisted development**: Commands can be run and outputs parsed programmatically
- **Testability**: Isolate and test features without GUI complexity
- **Debuggability**: Add `--verbose` to see exactly what's happening
- **Composability**: Script workflows, chain commands

**Pattern for new features:**

1. Implement core logic as a library function
2. Expose via `clap` CLI with `--verbose`, `--json`, `--dry-run` flags
3. Add `tracing` instrumentation at key decision points
4. Wire GUI as thin layer calling the same logic

### Audio Pipeline Architecture

```text
┌─────────────┐      ┌─────────────┐     ┌─────────────┐      ┌─────────────┐
│   Decoder   │────▶│  Resampler  │────▶│ Ring Buffer │────▶│ CPAL Output │
│  (Symphonia)│      │  (Rubato)   │     │   (rtrb)    │      │  (WASAPI)   │
└─────────────┘      └─────────────┘     └─────────────┘      └─────────────┘
      │                                        │                   │
      │ Decode Thread                          │ Lock-Free         │ RT Thread
      │ (normal priority)                      │ (no alloc)        │ (high priority)
      ▼                                        ▼                   ▼
┌─────────────┐                          ┌─────────────┐     ┌─────────────┐
│     FFT     │                          │AudioShared  │◀────│   Volume   │
│ Visualizer  │                          │   State     │     │  Control    │
└─────────────┘                          │ (atomics)   │     └─────────────┘
                                         └─────────────┘
                                               │
                                         ┌─────┴─────┐
                                         │ is_playing│
                                         │is_flushing│
                                         │ position  │
                                         │  volume   │
                                         └───────────┘
```

**Key guarantees:**

- No `Mutex`/`RwLock` in the audio callback — atomics only via `AudioSharedState`
- No heap allocation in the audio callback — ring buffer pre-allocated
- No blocking I/O in the audio callback — decoder runs in separate thread
- **Atomic flush mechanism** — when loading new track, `is_flushing` flag tells callback to drain buffer and output silence, preventing stale audio blips

## Tech Stack

- **Language**: Rust (2024 edition)
- **GUI Framework**: [Iced 0.13](https://github.com/iced-rs/iced) (Cross-platform, type-safe, Elm-inspired)
- **Audio Playback**: `cpal` (platform audio), `symphonia` (decoding), `rubato` (resampling)
- **Database**: SQLite via `sqlx` for library indexing
- **Audio Metadata**: `lofty` for reading/writing tags
- **Async Runtime**: `tokio`

### Core Modules

### Audio Pipeline (Critical Path)

1. **Player** (`player/mod.rs`): Orchestrates playback, queue management, command dispatch
2. **Audio Thread** (`player/audio.rs`): Decoding, resampling, ring buffer production, event emission
3. **Decoder** (`player/decoder.rs`): Symphonia-based format decoding
4. **Resampler** (`player/resampler.rs`): Rubato-based sample rate conversion (when device rate ≠ file rate)
5. **State** (`player/state.rs`): `AudioSharedState` (lock-free atomics), `PlayerState`, `PlayerEvent`, `PlayerCommand`
6. **Queue** (`player/queue.rs`): Playback queue with repeat modes, history tracking
7. **Visualization** (`player/visualization.rs`): FFT-based spectrum analyzer (decoupled from audio path)
8. **SIMD** (`player/simd.rs`): AVX2/SSE optimized volume scaling and format conversion

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
│   │   ├── mod.rs           # Player orchestration, command dispatch
│   │   ├── audio.rs         # Audio thread: decode, resample, ring buffer, events
│   │   ├── decoder.rs       # Symphonia format decoding
│   │   ├── resampler.rs     # Rubato sample rate conversion
│   │   ├── queue.rs         # Playback queue with repeat/shuffle
│   │   ├── state.rs         # AudioSharedState (atomics), PlayerState, events
│   │   ├── simd.rs          # AVX2/SSE optimized audio processing
│   │   ├── media_controls.rs# OS media key integration (SMTC/MPRIS)
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

The player uses an **event-driven architecture** where the UI sends commands to the audio thread, and the audio thread emits events back to confirm state changes. This eliminates race conditions between the UI and audio threads.

### Command → Event Flow

```text
┌─────────────────────────────────────────────────────────────────┐
│                        Entry Points                             │
├─────────────────────────────────────────────────────────────────┤
│ UI Button         → Message::PlayerPlay/Pause/Next/etc          │
│ OS Media Keys     → MediaControlPoll → MediaControlCommand      │
│ Keyboard Shortcut → Message::PlayerToggle/etc                   │
└──────────────────────────┬──────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                   ui/mod.rs - Message Router                    │
│  • MediaControlPoll: polls OS, emits MediaControlCommand        │
│  • PlayerTick: polls PlayerEvents from audio thread             │
│  • Routes all player messages to handle_player()                │
└──────────────────────────┬──────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│         ui/update/player.rs - handle_player()                   │
│  • MediaControlCommand converts to equivalent action            │
│  • Each action implemented ONCE via helper functions            │
│  • NO optimistic state updates - wait for events                │
└──────────────────────────┬──────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│              Internal Helpers (send commands only)              │
│  do_play()   → PlayerCommand::Play                              │
│  do_pause()  → PlayerCommand::Pause                             │
│  do_next()   → player.skip_forward() → Load + Play commands     │
│  do_seek()   → PlayerCommand::Seek                              │
│                                                                 │
│  NOTE: Helpers do NOT update UI state - they only send commands │
└──────────────────────────┬──────────────────────────────────────┘
                           │ crossbeam channel (lock-free)
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│              Audio Thread (player/audio.rs)                     │
│  • Receives PlayerCommand via channel                           │
│  • Processes command (load file, play, pause, seek)             │
│  • Emits PlayerEvent to confirm state change                    │
│  • Updates AudioSharedState atomics                             │
└──────────────────────────┬──────────────────────────────────────┘
                           │ crossbeam channel (lock-free)
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│              PlayerTick Subscription (100ms)                    │
│  • Calls player.poll_events() to drain event channel            │
│  • For each PlayerEvent:                                        │
│    - StatusChanged → update play/pause button state             │
│    - TrackLoaded → update track info, duration, cover art       │
│    - PlaybackFinished → auto-advance to next track              │
│    - Error → show error message                                 │
│  • Updates OS media controls (SMTC) with confirmed state        │
└─────────────────────────────────────────────────────────────────┘
```

### Event-Driven State Synchronization

The audio thread runs asynchronously. The UI **never reads shared state directly** — it only updates based on confirmed events from the audio thread.

**Why events, not polling `player.state()`?**

- **Race condition**: UI sends Play, immediately reads state → still shows "Stopped"
- **Stale data**: Audio thread hasn't processed command yet
- **Button flicker**: UI updates optimistically, then corrects when it reads real state

**Event-driven solution:**

1. UI sends `PlayerCommand::Play` to audio thread
2. UI does NOT update state (no optimistic updates)
3. Audio thread processes command, starts playback
4. Audio thread emits `PlayerEvent::StatusChanged(Playing)`
5. UI receives event via `PlayerTick` subscription
6. UI updates button to show "Pause" — state is now confirmed

**PlayerEvent types:**

| Event                                  | When Emitted              | UI Action                     |
| -------------------------------------- | ------------------------- | ----------------------------- |
| `StatusChanged(status)`                | Play/pause/stop processed | Update transport buttons      |
| `TrackLoaded { path, duration, ... }` | New file decoded          | Show track info, fetch cover  |
| `PositionChanged(duration)`            | Seek processed            | Update progress bar           |
| `PlaybackFinished`                     | Track ended naturally     | Auto-advance queue            |
| `Error(message)`                       | Decode/playback failure   | Show error toast              |

**Performance:** Events use `crossbeam_channel::try_send()` which is lock-free. The channel has 64 slots; if UI can't keep up, old events are dropped (acceptable since PlayerTick runs every 100ms).

### Track Transition: Atomic Flush

When skipping to a new track while paused, stale audio samples may remain in the ring buffer. Without handling, you'd hear a brief blip of the old track.

**Solution: Atomic flush flag in `AudioSharedState`**

```text
1. UI calls skip_forward() → sends Load + Play commands
2. Audio thread sets is_flushing = true
3. Audio callback sees is_flushing:
   - Drains all samples from ring buffer (consumer.pop() loop)
   - Outputs silence
4. Audio thread loads new decoder, resampler
5. Audio thread sets is_flushing = false
6. Audio callback resumes normal operation with new track's samples
```

This is lock-free (atomic bool) and handles the consumer side where stale samples actually live.

### Key Principles

1. **One implementation per action**: Play/pause/next/etc have a single code path
2. **No optimistic updates**: UI state only changes when audio thread confirms via event
3. **Lock-free communication**: Commands and events use crossbeam channels
4. **Atomic shared state**: `AudioSharedState` uses atomics for volume, position, flushing

### "Just Press Play" Behavior

When the user presses Play with an empty queue and no track loaded, the app automatically starts a random shuffle:

```text
PlayerPlay message received:
├── queue empty AND no track loaded → start_random_shuffle()
│   └── Pick 25 random tracks, queue them, skip_forward()
├── queue has tracks BUT no track loaded → skip_forward()
│   └── Start playing from existing queue
└── track already loaded → do_play()
    └── Resume playback
```

### Message Types

| Message | Source | Purpose |
|---------|--------|---------|
| `PlayerPlay/Pause/etc` | UI buttons | Direct user interaction |
| `MediaControlCommand` | OS media keys (via poll) | External control |
| `PlayerEvent` | Audio thread | Confirmed state changes |
| `PlayerTick` | Timer subscription (100ms) | Poll events + update position |
| `PlayerVisualizationTick` | Fast timer (16ms) | FFT data for visualizer |

### Debugging

To trace the command → event flow, enable debug logging:

```powershell
$env:RUST_LOG="ui::commands=debug,player::events=debug,ui::events=debug"
cargo run
```

**All log targets:**

| Target | Description | When to use |
|--------|-------------|-------------|
| `ui::commands` | UI command dispatch (`do_play()`, etc.) | Debug button clicks not working |
| `ui::events` | UI processing received events | Debug state not updating |
| `player::events` | Audio thread emitting events | Debug playback state changes |
| `scanner::progress` | File scanning progress | Debug slow/stuck scans |
| `enrichment::api` | MusicBrainz/AcoustID API calls | Debug metadata lookup failures |
| `cover::resolver` | Cover art resolution | Debug missing album art |

**Common debug scenarios:**

```powershell
# Play/pause not working
$env:RUST_LOG="ui::commands=debug,player::events=debug"

# Track not loading
$env:RUST_LOG="player::events=debug,player::decoder=debug"

# Metadata enrichment failing
$env:RUST_LOG="enrichment::api=debug"

# Full debug (verbose!)
$env:RUST_LOG="debug"

# With release build
$env:RUST_LOG="ui::commands=debug"; cargo run --release
```

**CLI debugging:**

Most features can be tested via CLI to isolate issues:

```powershell
# Test scanning
music-minder scan ./test_music --verbose

# Test identification
music-minder identify "track.mp3" --verbose

# Test enrichment (dry run)
music-minder enrich ./test_music --dry-run --verbose
```

### OS Media Controls (SMTC/MPRIS)

The OS media controls run on a dedicated thread (`media_controls.rs`) and communicate via channels:

- **Outbound** (to OS): metadata, playback state, position — sent when `TrackLoaded` event received
- **Inbound** (from OS): play/pause/next/prev/seek commands

Polling happens in `ui/mod.rs` via `MediaControlPoll` subscription (50ms interval). Commands are converted to `MediaControlCommand` messages and routed through `handle_player()`.

### Subscription Architecture

Iced uses an **async subscription system** for background tasks. Each subscription is a `futures::Stream` that emits messages to the UI. Critically, Iced's internal subscription tracker uses **bounded channels** — if a subscription produces messages faster than the UI can consume them, the channel fills up and events are dropped.

**Active Subscriptions:**

| Subscription | Interval | Purpose | Implementation |
|--------------|----------|---------|----------------|
| `PlayerTick` | 33ms (~30fps) | Poll player events, update progress bar | `time::every()` |
| `PlayerVisualizationTick` | 16ms (~60fps) | FFT data for spectrum analyzer | `time::every()` |
| `MediaControlPoll` | 50ms | Poll OS media key commands | `time::every()` |
| `WatcherStream` | N/A | File change notifications | **DISABLED** (see below) |

**Key Constraints:**

1. **No blocking in async streams**: Iced runs subscriptions cooperatively. A blocking call (like `recv_timeout()`) in one subscription starves ALL other subscriptions.
2. **Bounded internal channels**: Iced's subscription tracker drops events when its internal channel fills. Originally we used `window::frames()` (~60fps) for `PlayerTick`, which overflowed the channel. Reduced to 30fps via `time::every(33ms)`.
3. **CPU efficiency**: Faster tick rates consume more CPU. 30fps is sufficient for smooth progress bar updates.

**Watcher Subscription (Fixed)**

The file watcher subscription uses `tokio::sync::mpsc` with async `.recv().await` for non-blocking event polling. This allows other subscriptions (like `PlayerTick`) to continue firing normally.

**Architecture:**

- `FileWatcher::new_async()` returns a `tokio::sync::mpsc::Receiver<WatchEvent>`
- The `notify` callback uses `tx.try_send()` which is safe from sync contexts
- `watcher_stream()` uses `rx.recv().await` which yields to other async tasks
- The sync `FileWatcher::new()` with `crossbeam_channel` is preserved for CLI use

```rust
// Non-blocking watcher stream
pub fn watcher_stream(watch_paths: Vec<PathBuf>) -> impl futures::Stream<Item = Message> {
    futures::stream::unfold(WatcherStreamState::Init { watch_paths }, |state| async move {
        match state {
            WatcherStreamState::Running { _watcher, mut rx } => {
                // Non-blocking async receive - yields to other tasks while waiting
                match rx.recv().await {
                    Some(event) => Some((Message::WatcherEvent(event), ...)),
                    None => Some((Message::WatcherStopped, WatcherStreamState::Done)),
                }
            }
            // ...
        }
    })
}
```

## Mutation Philosophy

Rust requires explicit `&mut` for mutation. In this codebase:

**Justified mutation (required for performance or framework):**

- **Audio buffers** (resampler, FFT): Preallocated, reused every frame — allocating new buffers would cause glitches
- **Ring buffer**: Lock-free producer/consumer pattern requires mutable state
- **Play queue cursor**: Inherently stateful (current position in list)
- **UI state**: Iced's Elm architecture requires `&mut self` in `update()`

**Avoided mutation:**

- Read-only checks use immutable references (e.g., `queue().is_empty()` not `queue_mut().is_empty()`)
- Event-driven updates instead of polling mutable shared state

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
