//! Player state and command types.

use std::path::PathBuf;
use std::time::Duration;

/// Current playback status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaybackStatus {
    #[default]
    Stopped,
    Loading,
    Playing,
    Paused,
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
        format!(
            "{}Hz / {}ch / {}bit",
            self.sample_rate, self.channels, self.bits_per_sample
        )
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
