//! Integrated audio player with low-latency playback and visualization.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                      Player (Main Thread)                       │
//! │  Controls state, receives UI commands, updates visualization    │
//! └────────────────────────────┬────────────────────────────────────┘
//!                              │ crossbeam channels
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    Audio Thread (Real-time)                     │
//! │     Decodes audio, fills output buffer, sends FFT data          │
//! └────────────────────────────┬────────────────────────────────────┘
//!                              │ cpal callback
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                        WASAPI Output                            │
//! │              Low-latency audio to hardware                      │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Event-Driven State Synchronization
//!
//! The player uses events to communicate state changes back to the UI:
//!
//! 1. UI sends `PlayerCommand` (Play, Pause, etc.) via channel
//! 2. Audio thread processes command and updates actual state
//! 3. Audio thread emits `PlayerEvent` back to UI
//! 4. UI receives event via `poll_events()` and updates UI state
//!
//! This avoids race conditions from polling stale state after commands.
//!
//! # Debugging
//!
//! To see the full event flow, run with these log targets enabled:
//!
//! ```powershell
//! $env:RUST_LOG="player::events=debug,ui::commands=debug,ui::events=debug"
//! .\target\release\music-minder.exe
//! ```
//!
//! Log targets:
//! - `player::events` — Events emitted by audio thread (command processed)
//! - `ui::commands` — Commands sent by UI (button clicks)
//! - `ui::events` — Events received by UI (state updates)
//!
//! Example output:
//! ```text
//! DEBUG ui::commands: do_play() called
//! DEBUG player::events: Emit: StatusChanged(Playing)
//! DEBUG ui::events: Received StatusChanged: Stopped -> Playing
//! ```

mod audio;
mod decoder;
pub mod media_controls;
mod queue;
mod resampler;
pub mod simd;
mod state;
mod visualization;

pub use audio::{AudioConfig, AudioOutput};
pub use decoder::AudioDecoder;
pub use media_controls::{
    MediaControlCommand, MediaControlsHandle, MediaControlsMetadata, MediaPlaybackState,
};
pub use queue::{PlayQueue, QueueItem};
pub use resampler::Resampler;
pub use state::{AudioQuality, AudioSharedState, PlaybackStatus, PlayerCommand, PlayerEvent, PlayerState};
pub use visualization::{SpectrumData, VisualizationMode, Visualizer};

use crossbeam_channel::{Receiver, Sender, bounded};
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// The integrated audio player.
///
/// This is the main entry point for audio playback. It manages:
/// - Audio decoding (symphonia)
/// - Audio output (cpal/WASAPI)
/// - Play queue
/// - Visualization data
///
/// # Event-Driven Architecture
///
/// The player uses an event-driven model for state synchronization:
/// 1. UI sends commands via methods like `play()`, `pause()`, etc.
/// 2. Audio thread processes commands and emits `PlayerEvent`s
/// 3. UI calls `poll_events()` to receive confirmed state changes
///
/// This avoids race conditions from reading state immediately after commands.
pub struct Player {
    /// Current player state (shared with audio thread)
    state: Arc<RwLock<PlayerState>>,
    /// Lock-free shared state for the audio callback
    audio_shared: Option<Arc<AudioSharedState>>,
    /// Command sender to audio thread
    command_tx: Sender<PlayerCommand>,
    /// Event receiver from audio thread
    event_rx: Receiver<PlayerEvent>,
    /// Visualization data receiver from audio thread
    viz_rx: Receiver<SpectrumData>,
    /// The play queue
    queue: PlayQueue,
    /// Audio output handle
    _audio: Option<AudioOutput>,
}

impl Player {
    /// Create a new player instance.
    ///
    /// Returns `None` if audio output cannot be initialized.
    pub fn new() -> Option<Self> {
        let state = Arc::new(RwLock::new(PlayerState::default()));
        let (command_tx, command_rx) = bounded(32);
        let (event_tx, event_rx) = bounded(64); // Events from audio thread
        let (viz_tx, viz_rx) = bounded(4); // Small buffer, drop old frames

        // Try to initialize audio output
        let audio = AudioOutput::new(Arc::clone(&state), command_rx, event_tx, viz_tx).ok()?;
        let audio_shared = Some(Arc::clone(&audio.audio_shared));

        Some(Self {
            state,
            audio_shared,
            command_tx,
            event_rx,
            viz_rx,
            queue: PlayQueue::new(),
            _audio: Some(audio),
        })
    }

    /// Poll for events from the audio thread.
    ///
    /// Returns all pending events. This is the primary way for the UI to
    /// receive state change notifications. Call this in response to a
    /// subscription tick.
    pub fn poll_events(&self) -> Vec<PlayerEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.event_rx.try_recv() {
            events.push(event);
        }
        events
    }

    /// Load and play the current queue item.
    ///
    /// This is the SINGLE place that sends Load+Play commands to the audio thread.
    /// All playback initiation (play_file, skip_forward, previous) should use this.
    fn load_and_play_current(&mut self) -> Result<(), PlayerError> {
        if let Some(item) = self.queue.current() {
            self.command_tx
                .send(PlayerCommand::Load(item.path.clone()))
                .map_err(|_| PlayerError::ChannelClosed)?;
            self.command_tx
                .send(PlayerCommand::Play)
                .map_err(|_| PlayerError::ChannelClosed)?;
        }
        Ok(())
    }

    /// Play a file immediately (clears queue and starts playback).
    pub fn play_file(&mut self, path: PathBuf) -> Result<(), PlayerError> {
        self.queue.clear();
        self.queue.add(QueueItem::from_path(path));
        self.queue.jump_to(0);
        self.load_and_play_current()
    }

    /// Add a file to the queue.
    pub fn queue_file(&mut self, path: PathBuf) {
        self.queue.add(QueueItem::from_path(path));
    }

    /// Play the current track in the queue (loading it if necessary).
    ///
    /// If paused, resumes playback.
    /// If playing, does nothing.
    /// If stopped, reloads the current queue item and plays.
    pub fn play_current(&mut self) -> Result<(), PlayerError> {
        let status = self.state.read().status;
        if status == PlaybackStatus::Paused {
            self.play()
        } else if status == PlaybackStatus::Playing {
            Ok(()) // Already playing, do nothing
        } else {
            self.load_and_play_current()
        }
    }

    /// Play / resume playback.
    pub fn play(&self) -> Result<(), PlayerError> {
        tracing::debug!(target: "player::commands", "Player::play() - sending Play command");
        self.command_tx
            .send(PlayerCommand::Play)
            .map_err(|_| PlayerError::ChannelClosed)
    }

    /// Pause playback.
    pub fn pause(&self) -> Result<(), PlayerError> {
        tracing::debug!(target: "player::commands", "Player::pause() - sending Pause command");
        self.command_tx
            .send(PlayerCommand::Pause)
            .map_err(|_| PlayerError::ChannelClosed)
    }

    /// Toggle play/pause.
    pub fn toggle(&self) -> Result<(), PlayerError> {
        let status = self.state.read().status;
        tracing::debug!(
            target: "player::toggle",
            status = ?status,
            "Player::toggle() - deciding action based on current state"
        );
        match status {
            PlaybackStatus::Playing => {
                tracing::debug!(target: "player::toggle", "Sending Pause command");
                self.pause()
            }
            PlaybackStatus::Paused | PlaybackStatus::Stopped => {
                tracing::debug!(target: "player::toggle", "Sending Play command");
                self.play()
            }
            PlaybackStatus::Loading => {
                tracing::debug!(target: "player::toggle", "Ignoring toggle - currently Loading");
                Ok(())
            }
        }
    }

    /// Stop playback.
    pub fn stop(&self) -> Result<(), PlayerError> {
        self.command_tx
            .send(PlayerCommand::Stop)
            .map_err(|_| PlayerError::ChannelClosed)
    }

    /// Seek to a position (0.0 - 1.0).
    pub fn seek(&self, position: f32) -> Result<(), PlayerError> {
        self.command_tx
            .send(PlayerCommand::Seek(position.clamp(0.0, 1.0)))
            .map_err(|_| PlayerError::ChannelClosed)
    }

    /// Set volume (0.0 - 1.0).
    pub fn set_volume(&self, volume: f32) {
        let clamped = volume.clamp(0.0, 1.0);
        // Update UI state
        self.state.write().volume = clamped;
        // Update atomic state for real-time audio callback (lock-free)
        if let Some(ref audio_shared) = self.audio_shared {
            audio_shared.set_volume(clamped);
        }
    }

    /// Get current volume.
    pub fn volume(&self) -> f32 {
        self.state.read().volume
    }

    /// Skip to next track in queue.
    pub fn skip_forward(&mut self) -> Result<(), PlayerError> {
        if self.queue.skip_forward().is_some() {
            self.load_and_play_current()?;
        }
        Ok(())
    }

    /// Skip to previous track (or restart if > 3 seconds in).
    pub fn previous(&mut self) -> Result<(), PlayerError> {
        let position = self.state.read().position;
        if position > Duration::from_secs(3) {
            // Restart current track
            self.seek(0.0)
        } else if self.queue.previous().is_some() {
            self.load_and_play_current()
        } else {
            // At start of queue, just restart current track
            self.seek(0.0)
        }
    }

    /// Get current playback state snapshot.
    ///
    /// This syncs the position and underrun count from the atomic audio state.
    pub fn state(&self) -> PlayerState {
        let mut state = self.state.read().clone();
        // Sync position and underruns from atomic state (updated by audio callback)
        if let Some(ref audio_shared) = self.audio_shared {
            state.position = audio_shared.position();
            state.underruns = audio_shared.underruns();

            // Update quality metrics from real-time stats
            state.quality.buffer_fill = audio_shared.buffer_fill() as f32 / 100.0;

            // Estimate latency: ring buffer fill + typical WASAPI buffer (~10ms)
            // Ring buffer: 48000 samples at 48kHz stereo = ~500ms max
            // Current fill represents how much audio is buffered
            let buffer_latency_ms = state.quality.buffer_fill * 500.0;
            let wasapi_latency_ms = 10.0; // Typical WASAPI shared mode latency
            state.quality.latency_ms = buffer_latency_ms + wasapi_latency_ms;
        }
        state
    }

    /// Get audio performance statistics.
    pub fn performance_stats(&self) -> Option<AudioPerformanceStats> {
        self.audio_shared
            .as_ref()
            .map(|shared| AudioPerformanceStats {
                callback_count: shared.callback_count(),
                samples_processed: shared.samples_processed(),
                peak_callback_us: shared.peak_callback_us(),
                underruns: shared.underruns(),
                buffer_fill_percent: shared.buffer_fill(),
                simd_level: simd::current_simd_level().name(),
            })
    }

    /// Reset performance statistics.
    pub fn reset_stats(&self) {
        if let Some(ref audio_shared) = self.audio_shared {
            audio_shared.reset_stats();
        }
    }

    /// Get the latest visualization data (non-blocking).
    pub fn visualization(&self) -> Option<SpectrumData> {
        // Drain to get latest, return last
        let mut latest = None;
        while let Ok(data) = self.viz_rx.try_recv() {
            latest = Some(data);
        }
        latest
    }

    /// Get a reference to the play queue.
    pub fn queue(&self) -> &PlayQueue {
        &self.queue
    }

    /// Get a mutable reference to the play queue.
    pub fn queue_mut(&mut self) -> &mut PlayQueue {
        &mut self.queue
    }
}

impl Default for Player {
    fn default() -> Self {
        Self::new().expect("Failed to initialize audio output")
    }
}

/// List available audio output devices.
pub fn list_audio_devices() -> Vec<String> {
    use cpal::traits::{DeviceTrait, HostTrait};
    let host = cpal::default_host();
    host.output_devices()
        .map(|devices| devices.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default()
}

/// Get the current/default audio device name.
pub fn current_audio_device() -> String {
    use cpal::traits::{DeviceTrait, HostTrait};
    let host = cpal::default_host();
    host.default_output_device()
        .and_then(|d| d.name().ok())
        .unwrap_or_else(|| "Unknown".to_string())
}

/// Player errors.
#[derive(Debug, Clone, thiserror::Error)]
pub enum PlayerError {
    #[error("Audio output initialization failed: {0}")]
    AudioInit(String),

    #[error("Failed to decode audio: {0}")]
    Decode(String),

    #[error("Audio channel closed")]
    ChannelClosed,

    #[error("Unsupported audio format: {0}")]
    UnsupportedFormat(String),

    #[error("File not found: {0}")]
    FileNotFound(String),
}

/// Audio performance statistics for monitoring.
#[derive(Debug, Clone, Default)]
pub struct AudioPerformanceStats {
    /// Number of audio callbacks processed
    pub callback_count: u64,
    /// Total samples processed
    pub samples_processed: u64,
    /// Peak callback duration in microseconds
    pub peak_callback_us: u32,
    /// Number of buffer underruns
    pub underruns: u32,
    /// Current buffer fill percentage (0-100)
    pub buffer_fill_percent: u32,
    /// SIMD acceleration level in use
    pub simd_level: &'static str,
}

impl AudioPerformanceStats {
    /// Check if audio is performing well (no underruns, fast callbacks).
    pub fn is_healthy(&self) -> bool {
        self.underruns == 0 && self.peak_callback_us < 5000 // < 5ms
    }

    /// Get a health rating.
    pub fn health_rating(&self) -> &'static str {
        if self.underruns == 0 && self.peak_callback_us < 1000 {
            "Excellent"
        } else if self.underruns == 0 && self.peak_callback_us < 5000 {
            "Good"
        } else if self.underruns < 5 {
            "Fair"
        } else {
            "Poor"
        }
    }

    /// Get callback timing as a human-readable string.
    pub fn callback_timing(&self) -> String {
        if self.peak_callback_us < 1000 {
            format!("{}µs peak", self.peak_callback_us)
        } else {
            format!("{:.1}ms peak", self.peak_callback_us as f32 / 1000.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_state_default() {
        let state = PlayerState::default();
        assert_eq!(state.status, PlaybackStatus::Stopped);
        assert_eq!(state.volume, 1.0);
        assert_eq!(state.position, Duration::ZERO);
    }

    #[test]
    fn test_queue_operations() {
        let mut queue = PlayQueue::new();
        assert!(queue.is_empty());

        queue.add(QueueItem::from_path(PathBuf::from("test.mp3")));
        assert_eq!(queue.len(), 1);

        queue.clear();
        assert!(queue.is_empty());
    }
}
