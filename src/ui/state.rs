//! Application state types for the Music Minder UI.

use crate::{cover, db, diagnostics, enrichment, organizer, player};
use smallvec::SmallVec;
use sqlx::SqlitePool;
use std::path::PathBuf;

/// Top-level application state
///
/// Note: LoadedState is boxed to reduce stack size (Clippy large_enum_variant)
pub enum AppState {
    Loading,
    Loaded(Box<LoadedState>),
    Error(String),
}

/// The current view/mode of the organize panel
#[derive(Debug, Clone, Default, PartialEq)]
pub enum OrganizeView {
    #[default]
    Input, // Showing destination/pattern inputs
    Preview,    // Showing dry-run preview
    Organizing, // Currently organizing files
}

/// The active tab/pane in the main view
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ActivePane {
    #[default]
    Library,
    NowPlaying,
    Settings,
    Diagnostics,
}

/// Visualization mode for the player
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum VisualizationMode {
    #[default]
    Spectrum,
    Waveform,
    VuMeter,
    Off,
}

/// Virtualization constants - defined once, used everywhere
pub mod virtualization {
    /// Height of each track row in pixels
    pub const TRACK_ROW_HEIGHT: f32 = 30.0;
    /// Height of each preview row in pixels  
    pub const PREVIEW_ROW_HEIGHT: f32 = 18.0;
    /// Default viewport height when unknown
    pub const DEFAULT_VIEWPORT_HEIGHT: f32 = 400.0;
    /// Number of items to render above/below visible area for smooth scrolling
    pub const SCROLL_BUFFER: usize = 5;
}

/// State for a fully loaded application
pub struct LoadedState {
    pub pool: SqlitePool,

    // Active pane
    pub active_pane: ActivePane,

    // Scan state - PathBuf avoids repeated String->PathBuf conversions
    pub scan_path: PathBuf,
    pub is_scanning: bool,
    pub tracks: Vec<db::TrackWithMetadata>,
    pub tracks_loading: bool,
    pub status_message: String,
    pub scan_count: usize,

    // Scroll state for track list
    pub scroll_offset: f32,
    pub viewport_height: f32,

    // Scroll state for preview list
    pub preview_scroll_offset: f32,
    pub preview_viewport_height: f32,

    // Organize state - PathBuf for destination avoids conversions
    pub organize_destination: PathBuf,
    pub organize_pattern: String,
    pub organize_view: OrganizeView,
    pub organize_preview: Vec<organizer::OrganizePreview>,
    pub organize_progress: usize,
    pub organize_total: usize,
    // SmallVec: most organizes have 0-8 errors, avoid heap allocation
    pub organize_errors: SmallVec<[String; 8]>,
    pub can_undo: bool,
    pub preview_loading: bool,

    // Enrichment state
    pub enrichment: EnrichmentState,

    // Player state
    pub player: Option<player::Player>,
    pub player_state: player::PlayerState,
    pub visualization: player::SpectrumData,
    pub visualization_mode: VisualizationMode,
    pub auto_queue_enabled: bool,
    pub audio_devices: Vec<String>,
    pub current_audio_device: String,
    /// Seek preview position - when user is dragging the slider
    /// None = not seeking, Some(pos) = user is dragging to this position
    pub seek_preview: Option<f32>,

    // OS media controls (SMTC/MPRIS)
    pub media_controls: Option<player::MediaControlsHandle>,

    // Cover art state (non-blocking, resolved in background)
    pub cover_art: CoverArtState,

    // Diagnostics state
    pub diagnostics: Option<diagnostics::DiagnosticReport>,
    pub diagnostics_loading: bool,

    // Background file watcher state
    pub watcher_state: WatcherState,
}

impl LoadedState {
    /// Initialize player if not already done
    pub fn ensure_player(&mut self) {
        if self.player.is_none() {
            self.player = player::Player::new();
            if self.player.is_none() {
                self.status_message = "Failed to initialize audio output".to_string();
            }
        }
    }

    /// Find track metadata for the currently playing file
    pub fn current_track_info(&self) -> Option<&db::TrackWithMetadata> {
        let current_path = self.player_state.current_track.as_ref()?;
        let current_path_str = current_path.to_string_lossy();

        // Try exact match first
        if let Some(track) = self
            .tracks
            .iter()
            .find(|t| t.path == current_path_str.as_ref())
        {
            return Some(track);
        }

        // Try case-insensitive match (Windows paths)
        let current_lower = current_path_str.to_lowercase();
        self.tracks
            .iter()
            .find(|t| t.path.to_lowercase() == current_lower)
    }

    /// Find track metadata by path string
    pub fn track_info_by_path(&self, path: &std::path::Path) -> Option<&db::TrackWithMetadata> {
        let path_str = path.to_string_lossy();
        self.tracks.iter().find(|t| t.path == path_str.as_ref())
    }
}

/// State for the enrichment feature
#[derive(Default)]
pub struct EnrichmentState {
    /// AcoustID API key
    pub api_key: String,
    /// Currently selected track index (if any)
    pub selected_track: Option<usize>,
    /// Whether we're currently identifying a track
    pub is_identifying: bool,
    /// Result of last identification
    pub last_result: Option<enrichment::TrackIdentification>,
    /// Error message from last identification
    pub last_error: Option<String>,
    /// Whether fpcalc is available
    pub fpcalc_available: bool,
}

/// State for cover art display.
///
/// Cover art is resolved in the background to never block playback.
/// The UI displays whatever is available, gracefully degrading to
/// a placeholder if no art is found.
#[derive(Default)]
pub struct CoverArtState {
    /// Current cover art data (if available)
    pub current: Option<LoadedCoverArt>,
    /// Path of the track this cover is for (to detect stale data)
    pub for_track: Option<PathBuf>,
    /// Whether a fetch is in progress
    pub loading: bool,
    /// Error message if fetch failed (for debugging)
    pub error: Option<String>,
}

/// Cover art loaded and ready for display
#[derive(Debug, Clone)]
pub struct LoadedCoverArt {
    /// Raw image bytes
    pub data: Vec<u8>,
    /// MIME type (image/jpeg, image/png)
    pub mime_type: String,
    /// Source of this cover (embedded, sidecar, cached, remote)
    pub source: cover::CoverSource,
}

impl From<cover::CoverArt> for LoadedCoverArt {
    fn from(cover: cover::CoverArt) -> Self {
        Self {
            data: cover.data,
            mime_type: cover.mime_type,
            source: cover.source,
        }
    }
}

/// State for background file watching.
///
/// The watcher monitors the library directories and emits events when
/// files are added, modified, or removed. The UI can then trigger
/// incremental rescans without interrupting playback.
#[derive(Default)]
pub struct WatcherState {
    /// Whether the watcher is currently active
    pub active: bool,
    /// Directories being watched
    pub watch_paths: Vec<PathBuf>,
    /// Number of pending file changes (not yet processed)
    pub pending_changes: usize,
    /// Last error (if any)
    pub last_error: Option<String>,
}
