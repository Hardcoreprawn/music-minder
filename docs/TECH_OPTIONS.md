# Technology Stack Options

Since we are building a **Rust-based Windows Desktop Application** for music management, we have several strong candidates for the critical components.

## 1. GUI Framework
The choice of GUI determines the look, feel, and development experience.

### Option A: Iced (Recommended)
- **Type**: Retained mode (Elm Architecture).
- **Pros**: Pure Rust, type-safe, modern, cross-platform. Good balance of control and ease of use.
- **Cons**: "Game-like" rendering (wgpu), not native Windows controls (though can be styled).
- **Verdict**: Excellent for a custom, modern-looking app.

### Option B: Egui
- **Type**: Immediate mode.
- **Pros**: Extremely fast, easiest to set up, very productive for tools/editors.
- **Cons**: Non-standard look (very utilitarian/custom), high CPU usage (redraws every frame usually, though optimized).
- **Verdict**: Best if we want to move *fast* and don't care about native Windows aesthetics.

### Option C: Tauri
- **Type**: Webview (Rust backend + HTML/JS frontend).
- **Pros**: Native Windows look (via CSS/libraries), huge ecosystem (React/Svelte).
- **Cons**: Requires web tech (JS/TS) for the UI.
- **Verdict**: Disqualified based on "stick to Rust" preference, unless you are okay with HTML/CSS for UI.

### Option D: Slint
- **Type**: UI Toolkit (Custom DSL).
- **Pros**: Lightweight, native-ish feel, designed for embedded and desktop.
- **Cons**: Learning a separate DSL (.slint files).

## 2. Audio Metadata
We need to read and write tags (ID3, Vorbis, etc.).

### Option A: Lofty (Recommended)
- **Pros**: High-level, supports a vast array of formats (MP3, FLAC, OGG, MP4), active development. Focuses specifically on tagging.
- **Cons**: None significant for this use case.

### Option B: Symphonia
- **Pros**: Great for *decoding* and playback.
- **Cons**: Metadata is secondary; writing tags is less mature than Lofty.

### Option C: id3 / mp4ameta (Individual crates)
- **Pros**: Specialized.
- **Cons**: Managing multiple crates for different formats is painful.

## 3. Database
To store the library index (thousands of tracks).

### Option A: SQLite (via `sqlx`) (Recommended)
- **Pros**: Async, compile-time query verification, relational (easy to do "Select * from tracks where artist='X'").
- **Cons**: Slightly heavier setup than raw rusqlite.

### Option B: SQLite (via `rusqlite`)
- **Pros**: Simple, synchronous (blocking).
- **Cons**: Blocking I/O can freeze the UI if not handled carefully in threads.

### Option C: Sled (Embedded Key-Value)
- **Pros**: Pure Rust, fast.
- **Cons**: No SQL. Complex queries (filtering/sorting) must be done in code.

## Recommendation
**Stack**: **Iced** (UI) + **Lofty** (Metadata) + **SQLx** (SQLite).
This provides a modern, type-safe, pure Rust experience with powerful querying capabilities.
