//! Player state and command types.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::time::Duration;

/// Lock-free shared state for the audio callback.
///
/// This struct uses atomics to avoid priority inversion in the real-time audio thread.
/// The cpal callback runs on a high-priority system thread and must never block on locks.
#[derive(Debug)]
pub struct AudioSharedState {
    /// Volume as f32 bits (use `f32::to_bits()` / `f32::from_bits()`)
    volume_bits: AtomicU32,
    /// Whether playback is active
    is_playing: AtomicBool,
    /// Whether the buffer is being flushed (drain old samples, output silence)
    is_flushing: AtomicBool,
    /// Current position in nanoseconds
    position_nanos: AtomicU64,
    /// Buffer underrun count
    underruns: AtomicU32,
    /// Callback invocation count (for latency calculation)
    callback_count: AtomicU64,
    /// Total samples processed
    samples_processed: AtomicU64,
    /// Peak callback duration in microseconds
    peak_callback_us: AtomicU32,
    /// Ring buffer fill level (0-100)
    buffer_fill_percent: AtomicU32,
}

impl Default for AudioSharedState {
    fn default() -> Self {
        Self {
            volume_bits: AtomicU32::new(1.0_f32.to_bits()),
            is_playing: AtomicBool::new(false),
            is_flushing: AtomicBool::new(false),
            position_nanos: AtomicU64::new(0),
            underruns: AtomicU32::new(0),
            callback_count: AtomicU64::new(0),
            samples_processed: AtomicU64::new(0),
            peak_callback_us: AtomicU32::new(0),
            buffer_fill_percent: AtomicU32::new(0),
        }
    }
}

impl AudioSharedState {
    /// Create a new audio shared state wrapped in Arc.
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Get the current volume (0.0 - 1.0).
    #[inline]
    pub fn volume(&self) -> f32 {
        f32::from_bits(self.volume_bits.load(Ordering::Relaxed))
    }

    /// Set the volume (0.0 - 1.0).
    #[inline]
    pub fn set_volume(&self, volume: f32) {
        self.volume_bits
            .store(volume.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }

    /// Check if playback is active.
    #[inline]
    pub fn is_playing(&self) -> bool {
        self.is_playing.load(Ordering::Relaxed)
    }

    /// Set the playing state.
    #[inline]
    pub fn set_playing(&self, playing: bool) {
        self.is_playing.store(playing, Ordering::Relaxed);
    }

    /// Check if buffer is being flushed.
    #[inline]
    pub fn is_flushing(&self) -> bool {
        self.is_flushing.load(Ordering::Acquire)
    }

    /// Start flushing - audio callback will drain buffer and output silence.
    #[inline]
    pub fn start_flush(&self) {
        self.is_flushing.store(true, Ordering::Release);
    }

    /// Stop flushing - audio callback resumes normal operation.
    #[inline]
    pub fn stop_flush(&self) {
        self.is_flushing.store(false, Ordering::Release);
    }

    /// Get the current position as Duration.
    #[inline]
    pub fn position(&self) -> Duration {
        Duration::from_nanos(self.position_nanos.load(Ordering::Relaxed))
    }

    /// Set the current position.
    #[inline]
    pub fn set_position(&self, position: Duration) {
        self.position_nanos
            .store(position.as_nanos() as u64, Ordering::Relaxed);
    }

    /// Get the underrun count.
    #[inline]
    pub fn underruns(&self) -> u32 {
        self.underruns.load(Ordering::Relaxed)
    }

    /// Increment the underrun count (returns new value).
    #[inline]
    pub fn increment_underruns(&self) -> u32 {
        self.underruns.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Record a callback completion with timing.
    #[inline]
    pub fn record_callback(&self, samples: u32, duration_us: u32) {
        self.callback_count.fetch_add(1, Ordering::Relaxed);
        self.samples_processed
            .fetch_add(samples as u64, Ordering::Relaxed);

        // Update peak if this callback was slower
        let mut current_peak = self.peak_callback_us.load(Ordering::Relaxed);
        while duration_us > current_peak {
            match self.peak_callback_us.compare_exchange_weak(
                current_peak,
                duration_us,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(c) => current_peak = c,
            }
        }
    }

    /// Set the buffer fill percentage (0-100).
    #[inline]
    pub fn set_buffer_fill(&self, percent: u32) {
        self.buffer_fill_percent
            .store(percent.min(100), Ordering::Relaxed);
    }

    /// Get the buffer fill percentage.
    #[inline]
    pub fn buffer_fill(&self) -> u32 {
        self.buffer_fill_percent.load(Ordering::Relaxed)
    }

    /// Get the callback count.
    #[inline]
    pub fn callback_count(&self) -> u64 {
        self.callback_count.load(Ordering::Relaxed)
    }

    /// Get total samples processed.
    #[inline]
    pub fn samples_processed(&self) -> u64 {
        self.samples_processed.load(Ordering::Relaxed)
    }

    /// Get peak callback duration in microseconds.
    #[inline]
    pub fn peak_callback_us(&self) -> u32 {
        self.peak_callback_us.load(Ordering::Relaxed)
    }

    /// Reset performance counters.
    pub fn reset_stats(&self) {
        self.underruns.store(0, Ordering::Relaxed);
        self.callback_count.store(0, Ordering::Relaxed);
        self.samples_processed.store(0, Ordering::Relaxed);
        self.peak_callback_us.store(0, Ordering::Relaxed);
    }
}

/// Current playback status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaybackStatus {
    #[default]
    Stopped,
    Loading,
    Playing,
    Paused,
}

/// Audio format quality information.
#[derive(Debug, Clone, Default)]
pub struct AudioQuality {
    /// Source format (e.g., "FLAC", "MP3 320kbps")
    pub format: String,
    /// Whether the source is lossless
    pub is_lossless: bool,
    /// Source bit depth (16, 24, 32)
    pub bit_depth: u16,
    /// Source sample rate
    pub source_sample_rate: u32,
    /// Output sample rate (may differ if resampling)
    pub output_sample_rate: u32,
    /// Whether bit-perfect playback is achieved
    pub is_bit_perfect: bool,
    /// Estimated end-to-end latency in milliseconds
    pub latency_ms: f32,
    /// Ring buffer size in samples
    pub buffer_size: usize,
    /// Current buffer fill level (0.0-1.0)
    pub buffer_fill: f32,
}

impl AudioQuality {
    /// Get a human-readable quality description.
    pub fn quality_label(&self) -> &'static str {
        if self.is_lossless && self.bit_depth >= 24 && self.source_sample_rate >= 96000 {
            "Hi-Res Lossless"
        } else if self.is_lossless && self.bit_depth >= 24 {
            "Lossless 24-bit"
        } else if self.is_lossless {
            "Lossless"
        } else {
            "Lossy"
        }
    }

    /// Get the quality tier emoji.
    pub fn quality_emoji(&self) -> &'static str {
        if self.is_lossless && self.bit_depth >= 24 {
            "🎵" // Hi-res
        } else if self.is_lossless {
            "💿" // CD quality
        } else {
            "🎧" // Lossy
        }
    }
}

/// Shared player state.
#[derive(Debug, Clone)]
pub struct PlayerState {
    /// Current playback status
    pub status: PlaybackStatus,
    /// Current track path (if any)
    pub current_track: Option<PathBuf>,
    /// Current position in the track
    pub position: Duration,
    /// Total duration of the track
    pub duration: Duration,
    /// Volume level (0.0 - 1.0)
    pub volume: f32,
    /// Sample rate of current track
    pub sample_rate: u32,
    /// Number of channels
    pub channels: u16,
    /// Current bit depth
    pub bits_per_sample: u16,
    /// Buffer underrun count (for diagnostics)
    pub underruns: u32,
    /// Audio quality information
    pub quality: AudioQuality,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            status: PlaybackStatus::Stopped,
            current_track: None,
            position: Duration::ZERO,
            duration: Duration::ZERO,
            volume: 1.0,
            sample_rate: 44100,
            channels: 2,
            bits_per_sample: 16,
            underruns: 0,
            quality: AudioQuality::default(),
        }
    }
}

impl PlayerState {
    /// Get position as a fraction (0.0 - 1.0).
    pub fn position_fraction(&self) -> f32 {
        if self.duration.is_zero() {
            0.0
        } else {
            self.position.as_secs_f32() / self.duration.as_secs_f32()
        }
    }

    /// Format position as MM:SS.
    pub fn position_str(&self) -> String {
        format_duration(self.position)
    }

    /// Format duration as MM:SS.
    pub fn duration_str(&self) -> String {
        format_duration(self.duration)
    }

    /// Get a display string for the current format.
    pub fn format_info(&self) -> String {
        let quality_indicator = if self.quality.is_lossless {
            "Lossless"
        } else {
            "Lossy"
        };

        format!(
            "{} • {}Hz / {}ch / {}bit • {}",
            self.quality.format,
            self.sample_rate,
            self.channels,
            self.bits_per_sample,
            quality_indicator
        )
    }

    /// Get a compact debug summary for logging.
    /// Format: "Status@Pos/Dur" e.g. "Playing@1:23/4:56"
    pub fn debug_summary(&self) -> String {
        let status = match self.status {
            PlaybackStatus::Stopped => "Stopped",
            PlaybackStatus::Playing => "Playing",
            PlaybackStatus::Paused => "Paused",
            PlaybackStatus::Loading => "Loading",
        };
        format!("{}@{}/{}", status, self.position_str(), self.duration_str())
    }
}

/// Format a duration as MM:SS or HH:MM:SS.
pub fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    let secs = secs % 60;

    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, mins, secs)
    } else {
        format!("{}:{:02}", mins, secs)
    }
}

/// Commands sent to the audio thread.
#[derive(Debug, Clone)]
pub enum PlayerCommand {
    /// Load a new file
    Load(PathBuf),
    /// Start/resume playback
    Play,
    /// Pause playback
    Pause,
    /// Stop playback
    Stop,
    /// Seek to position (0.0 - 1.0)
    Seek(f32),
    /// Shutdown the audio thread
    Shutdown,
}

/// Events sent from the audio thread to notify the UI of state changes.
///
/// This enables an event-driven architecture where:
/// 1. UI sends commands via `PlayerCommand`
/// 2. Audio thread processes commands and emits events
/// 3. UI receives events and updates state (single source of truth)
///
/// This avoids race conditions from polling stale state after sending commands.
#[derive(Debug, Clone)]
pub enum PlayerEvent {
    /// Playback status changed
    StatusChanged(PlaybackStatus),
    /// A new track was loaded with its metadata
    TrackLoaded {
        path: PathBuf,
        duration: Duration,
        sample_rate: u32,
        channels: u16,
        bits_per_sample: u16,
        quality: AudioQuality,
    },
    /// Position updated (sent periodically during playback)
    PositionChanged(Duration),
    /// Playback finished (end of track)
    PlaybackFinished,
    /// An error occurred
    Error(String),
}

/// Track metadata for display.
#[derive(Debug, Clone, Default)]
pub struct TrackInfo {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub track_number: Option<u32>,
    pub year: Option<i32>,
    pub genre: Option<String>,
}

impl TrackInfo {
    /// Get display title (filename if no title tag).
    pub fn display_title(&self, path: &std::path::Path) -> String {
        self.title.clone().unwrap_or_else(|| {
            path.file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "Unknown".to_string())
        })
    }

    /// Get display artist.
    pub fn display_artist(&self) -> &str {
        self.artist.as_deref().unwrap_or("Unknown Artist")
    }

    /// Get display album.
    pub fn display_album(&self) -> &str {
        self.album.as_deref().unwrap_or("Unknown Album")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(0)), "0:00");
        assert_eq!(format_duration(Duration::from_secs(65)), "1:05");
        assert_eq!(format_duration(Duration::from_secs(3661)), "1:01:01");
    }

    #[test]
    fn test_position_fraction() {
        let mut state = PlayerState::default();
        assert_eq!(state.position_fraction(), 0.0);

        state.duration = Duration::from_secs(100);
        state.position = Duration::from_secs(50);
        assert!((state.position_fraction() - 0.5).abs() < 0.01);
    }
}
