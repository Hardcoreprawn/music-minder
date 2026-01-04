//! # Symphonium
//!
//! Audio playback and decoding pipeline for Music Minder.
//!
//! This crate provides a modular, lock-free audio architecture supporting:
//! - Format decoding (MP3, FLAC, OGG, WAV, AAC) via Symphonia
//! - Sample rate conversion via Rubato
//! - Real-time audio output via CPAL (WASAPI on Windows, CoreAudio on macOS, ALSA on Linux)
//! - Visualization (FFT, spectrum analysis) for real-time audio visualization
//! - Media controls integration (SMTC on Windows, MPRIS on Linux)
//!
//! # Architecture
//!
//! The audio pipeline is event-driven and fully async:
//!
//! ```text
//! UI (Iced)
//!   ↓ (PlayerCommand)
//! [crossbeam channel]
//!   ↓
//! Audio Thread (real-time)
//!   ├→ Decoder (Symphonia)
//!   ├→ Resampler (Rubato)
//!   ├→ CPAL Output (ring buffer)
//!   └→ FFT Analyzer
//!   ↑ (PlayerEvent + Visualization)
//! [crossbeam channel]
//!   ↓
//! UI (poll_events)
//! ```
//!
//! # Real-time Safety
//!
//! The CPAL callback runs on a high-priority audio thread. To avoid glitches:
//! - No locks (RwLock/Mutex) in the hot path
//! - Lock-free ring buffers for sample data (rtrb)
//! - Atomic operations for state updates (AudioSharedState)
//! - No allocations or blocking operations

pub mod player;

// Re-export key types for easy access
pub use player::{
    AudioConfig, AudioDecoder, AudioOutput, AudioPerformanceStats, AudioQuality,
    MediaControlCommand, MediaControlsHandle, MediaControlsMetadata, MediaPlaybackState, PlayQueue,
    PlaybackStatus, Player, PlayerCommand, PlayerEvent, PlayerState, QueueItem, RepeatMode,
    Resampler, SpectrumData, TrackInfo, VisualizationMode, Visualizer, format_duration,
    format_duration_secs, list_audio_devices,
};

// Re-export submodules that are used elsewhere
pub use player::simd;
