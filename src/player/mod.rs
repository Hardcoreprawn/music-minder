//! Integrated audio player with low-latency playback and visualization.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                      Player (Main Thread)                       │
//! │  Controls state, receives UI commands, updates visualization   │
//! └────────────────────────────┬────────────────────────────────────┘
//!                              │ crossbeam channels
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    Audio Thread (Real-time)                     │
//! │     Decodes audio, fills output buffer, sends FFT data         │
//! └────────────────────────────┬────────────────────────────────────┘
//!                              │ cpal callback
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                        WASAPI Output                            │
//! │              Low-latency audio to hardware                      │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

mod audio;
mod decoder;
mod queue;
mod state;
mod visualization;

pub use audio::{AudioOutput, AudioConfig};
pub use decoder::AudioDecoder;
pub use queue::{PlayQueue, QueueItem};
pub use state::{PlayerState, PlaybackStatus, PlayerCommand};
pub use visualization::{SpectrumData, VisualizationMode, Visualizer};

use crossbeam_channel::{Receiver, Sender, bounded};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use parking_lot::RwLock;

/// The integrated audio player.
///
/// This is the main entry point for audio playback. It manages:
/// - Audio decoding (symphonia)
/// - Audio output (cpal/WASAPI)
/// - Play queue
/// - Visualization data
pub struct Player {
    /// Current player state (shared with audio thread)
    state: Arc<RwLock<PlayerState>>,
    /// Command sender to audio thread
    command_tx: Sender<PlayerCommand>,
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
        let (viz_tx, viz_rx) = bounded(4); // Small buffer, drop old frames
        
        // Try to initialize audio output
        let audio = AudioOutput::new(
            Arc::clone(&state),
            command_rx,
            viz_tx,
        ).ok()?;
        
        Some(Self {
            state,
            command_tx,
            viz_rx,
            queue: PlayQueue::new(),
            _audio: Some(audio),
        })
    }

    /// Play a file immediately (clears queue and starts playback).
    pub fn play_file(&mut self, path: PathBuf) -> Result<(), PlayerError> {
        self.queue.clear();
        self.queue.add(QueueItem::from_path(path.clone()));
        self.command_tx.send(PlayerCommand::Load(path))
            .map_err(|_| PlayerError::ChannelClosed)?;
        self.command_tx.send(PlayerCommand::Play)
            .map_err(|_| PlayerError::ChannelClosed)?;
        Ok(())
    }

    /// Add a file to the queue.
    pub fn queue_file(&mut self, path: PathBuf) {
        self.queue.add(QueueItem::from_path(path));
    }

    /// Play / resume playback.
    pub fn play(&self) -> Result<(), PlayerError> {
        self.command_tx.send(PlayerCommand::Play)
            .map_err(|_| PlayerError::ChannelClosed)
    }

    /// Pause playback.
    pub fn pause(&self) -> Result<(), PlayerError> {
        self.command_tx.send(PlayerCommand::Pause)
            .map_err(|_| PlayerError::ChannelClosed)
    }

    /// Toggle play/pause.
    pub fn toggle(&self) -> Result<(), PlayerError> {
        let status = self.state.read().status;
        match status {
            PlaybackStatus::Playing => self.pause(),
            PlaybackStatus::Paused | PlaybackStatus::Stopped => self.play(),
            PlaybackStatus::Loading => Ok(()),
        }
    }

    /// Stop playback.
    pub fn stop(&self) -> Result<(), PlayerError> {
        self.command_tx.send(PlayerCommand::Stop)
            .map_err(|_| PlayerError::ChannelClosed)
    }

    /// Seek to a position (0.0 - 1.0).
    pub fn seek(&self, position: f32) -> Result<(), PlayerError> {
        self.command_tx.send(PlayerCommand::Seek(position.clamp(0.0, 1.0)))
            .map_err(|_| PlayerError::ChannelClosed)
    }

    /// Set volume (0.0 - 1.0).
    pub fn set_volume(&self, volume: f32) {
        self.state.write().volume = volume.clamp(0.0, 1.0);
    }

    /// Get current volume.
    pub fn volume(&self) -> f32 {
        self.state.read().volume
    }

    /// Skip to next track in queue.
    pub fn skip_forward(&mut self) -> Result<(), PlayerError> {
        if let Some(item) = self.queue.skip_forward() {
            self.command_tx.send(PlayerCommand::Load(item.path.clone()))
                .map_err(|_| PlayerError::ChannelClosed)?;
            self.command_tx.send(PlayerCommand::Play)
                .map_err(|_| PlayerError::ChannelClosed)?;
        }
        Ok(())
    }

    /// Skip to previous track (or restart if > 3 seconds in).
    pub fn previous(&mut self) -> Result<(), PlayerError> {
        let position = self.state.read().position;
        if position > Duration::from_secs(3) {
            // Restart current track
            self.seek(0.0)
        } else if let Some(item) = self.queue.previous() {
            self.command_tx.send(PlayerCommand::Load(item.path.clone()))
                .map_err(|_| PlayerError::ChannelClosed)?;
            self.command_tx.send(PlayerCommand::Play)
                .map_err(|_| PlayerError::ChannelClosed)?;
            Ok(())
        } else {
            self.seek(0.0)
        }
    }

    /// Get current playback state snapshot.
    pub fn state(&self) -> PlayerState {
        self.state.read().clone()
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
    use cpal::traits::{HostTrait, DeviceTrait};
    let host = cpal::default_host();
    host.output_devices()
        .map(|devices| {
            devices
                .filter_map(|d| d.name().ok())
                .collect()
        })
        .unwrap_or_default()
}

/// Get the current/default audio device name.
pub fn current_audio_device() -> String {
    use cpal::traits::{HostTrait, DeviceTrait};
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
