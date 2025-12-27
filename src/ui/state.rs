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
    Enrich,
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

/// Which list currently has keyboard focus for navigation
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FocusedList {
    #[default]
    Library,
    Queue,
}

/// State for drag-and-drop reordering in the queue
#[derive(Debug, Clone, Default)]
#[allow(dead_code)] // Fields will be used when drag-drop is fully implemented
pub struct QueueDragState {
    /// Item currently being dragged (if any)
    pub dragging: Option<DragInfo>,
    /// Current drop target index (for insertion line visual)
    pub drop_target: Option<usize>,
}

/// Information about an item being dragged
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields will be used when drag-drop is fully implemented
pub struct DragInfo {
    /// Original index of the dragged item in the queue
    pub index: usize,
    /// Y position at drag start (captured on first move event)
    pub origin_y: Option<f32>,
    /// Current cursor Y position
    pub current_y: f32,
    /// Snapshot of shuffle state at drag start
    pub is_shuffle_mode: bool,
}

impl std::fmt::Display for VisualizationMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VisualizationMode::Spectrum => write!(f, "Spectrum"),
            VisualizationMode::Waveform => write!(f, "Waveform"),
            VisualizationMode::VuMeter => write!(f, "VU Meter"),
            VisualizationMode::Off => write!(f, "Off"),
        }
    }
}

/// Column to sort the library by
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SortColumn {
    #[default]
    Title,
    Artist,
    Album,
    Year,
    Duration,
    Format,
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

    // Search and filter state
    pub search_query: String,
    pub filtered_indices: Vec<usize>, // Indices into `tracks` that match search/filters
    pub sort_column: SortColumn,
    pub sort_ascending: bool,
    pub filter_format: Option<String>, // None = all formats, Some("FLAC") = only FLAC
    pub filter_lossless: Option<bool>, // None = all, Some(true) = lossless only

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

    // Enrichment pane state (batch operations)
    pub enrichment_pane: EnrichmentPaneState,

    // Player state
    pub player: Option<player::Player>,
    pub player_state: player::PlayerState,
    /// Metadata read from file tags (fallback when track not in DB)
    pub file_metadata: Option<player::TrackInfo>,
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
    pub diagnostics_started_tick: u32,
    /// Pending result waiting for animation to complete
    pub diagnostics_pending: Option<diagnostics::DiagnosticReport>,
    /// Which diagnostic checks are expanded (by check name)
    pub diagnostics_expanded: std::collections::HashSet<String>,

    /// High resolution timer guard - requests 1ms timer while app runs
    /// This improves audio scheduling precision on Windows
    #[cfg(windows)]
    /// High resolution timer for precise timing (reserved for future use)
    #[allow(dead_code)]
    pub high_res_timer: Option<diagnostics::HighResolutionTimer>,

    // Animation tick counter (incremented by PlayerTick at 60fps)
    // Used for spinner animations and other subtle UI animations
    pub animation_tick: u32,

    // Background file watcher state
    pub watcher_state: WatcherState,

    // Background quality gardener state
    pub gardener_state: GardenerState,

    // Sidebar state
    pub sidebar_collapsed: bool,

    // Organize section collapsed state
    pub organize_collapsed: bool,

    // Selection tracking for keyboard navigation
    /// Which list has keyboard focus (Library or Queue)
    pub focused_list: FocusedList,
    /// Selected index in the library list (into filtered_indices or tracks)
    pub library_selection: Option<usize>,
    /// Selected index in the queue list
    pub queue_selection: Option<usize>,

    // Queue drag-and-drop state
    #[allow(dead_code)] // Will be used when drag-drop UI is fully implemented
    pub queue_drag: QueueDragState,

    // Easter egg state for empty album art placeholder
    pub easter_egg_index: usize,
    pub easter_egg_clicks: u32,

    // Track detail modal state
    pub track_detail: TrackDetailState,
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

    /// Get display info for the current track using fallback chain.
    ///
    /// Priority: 1. Database metadata → 2. File tags → 3. Filename
    ///
    /// Returns (title, artist, album) with best available info.
    pub fn current_track_display(&self) -> Option<(String, String, String)> {
        let current_path = self.player_state.current_track.as_ref()?;

        // 1. Try database first (most complete metadata)
        if let Some(track) = self.current_track_info() {
            return Some((
                track.title.clone(),
                track.artist_name.clone(),
                track.album_name.clone(),
            ));
        }

        // 2. Try file tags (from decoder)
        if let Some(ref file_meta) = self.file_metadata {
            let has_any_metadata = file_meta.title.is_some()
                || file_meta.artist.is_some()
                || file_meta.album.is_some();

            if has_any_metadata {
                // Build display with fallbacks within file metadata
                let title = file_meta.title.clone().unwrap_or_else(|| {
                    current_path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "Unknown".to_string())
                });
                let artist = file_meta.artist.clone().unwrap_or_default();
                let album = file_meta.album.clone().unwrap_or_default();

                return Some((title, artist, album));
            }
        }

        // 3. Fall back to filename
        let title = current_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        Some((title, String::new(), String::new()))
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
    /// Whether the API key has been saved (for UI feedback)
    pub api_key_saved: bool,
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

/// State for track detail modal view
#[derive(Default)]
pub struct TrackDetailState {
    /// Track index being viewed (into LoadedState.tracks)
    pub track_index: Option<usize>,
    /// Fresh metadata read from the file (may differ from DB if changed)
    pub file_metadata: Option<crate::metadata::TrackMetadata>,
    /// Full metadata from file (all fields)
    pub full_metadata: Option<crate::metadata::FullMetadata>,
    /// File format info
    pub format_info: Option<FileFormatInfo>,
    /// Whether we're currently identifying the track
    pub is_identifying: bool,
    /// Identification result (if any)
    pub identification: Option<enrichment::TrackIdentification>,
    /// Error message (if identification failed)
    pub error: Option<String>,
    /// Whether tags were recently written
    pub tags_written: bool,
}

/// Audio file format information
#[derive(Debug, Clone)]
pub struct FileFormatInfo {
    /// File extension (mp3, flac, etc.)
    pub extension: String,
    /// Bitrate if available
    pub bitrate: Option<u32>,
    /// Sample rate if available
    pub sample_rate: Option<u32>,
    /// Channels (1=mono, 2=stereo)
    pub channels: Option<u8>,
    /// Whether lossless format
    pub is_lossless: bool,
}

/// State for the enrichment pane (batch operations)
#[derive(Default)]
pub struct EnrichmentPaneState {
    /// AcoustID API key (shared with EnrichmentState)
    pub api_key: String,
    /// Whether fpcalc is available
    pub fpcalc_available: bool,
    /// Rate limit status for display
    pub rate_limit_status: RateLimitStatus,

    /// Track indices selected for enrichment (indices into LoadedState.tracks)
    pub selected_tracks: Vec<usize>,
    /// Which tracks in selected_tracks are checked for processing
    pub checked_tracks: std::collections::HashSet<usize>,

    /// Options
    pub fill_only: bool,
    pub fetch_cover_art: bool,

    /// Whether batch identification is in progress
    pub is_identifying: bool,
    /// Results of identification
    pub results: Vec<EnrichmentResult>,
}

impl EnrichmentPaneState {
    /// Check if there are confirmed results ready to write
    pub fn has_confirmed_results(&self) -> bool {
        self.results
            .iter()
            .any(|r| r.confirmed && r.status == ResultStatus::Success)
    }
}

/// Rate limit status for display
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum RateLimitStatus {
    #[default]
    Ok,
    Warning,
    Limited,
}

/// Result status for enrichment
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ResultStatus {
    #[default]
    Pending,
    Success,
    Warning,
    Error,
}

/// A single enrichment result
#[derive(Debug, Clone, Default)]
pub struct EnrichmentResult {
    /// Track index in selected_tracks
    pub track_index: usize,
    /// Status of this result
    pub status: ResultStatus,
    /// Identified title (if found)
    pub title: Option<String>,
    /// Identified artist (if found)
    pub artist: Option<String>,
    /// Identified album (if found)
    pub album: Option<String>,
    /// Match confidence (0.0 - 1.0)
    pub confidence: Option<f32>,
    /// List of fields that would change
    pub changes: Vec<String>,
    /// Error message (if status is Error)
    pub error: Option<String>,
    /// Whether this result is confirmed for writing
    pub confirmed: bool,
    /// Full identification result for writing
    pub identification: Option<crate::enrichment::TrackIdentification>,
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

/// State for the background quality gardener.
#[derive(Default)]
pub struct GardenerState {
    /// Whether the gardener is running
    pub active: bool,
    /// Command sender for triggering quality checks
    pub command_tx: Option<tokio::sync::mpsc::Sender<crate::health::GardenerCommand>>,
    /// Tracks assessed in current session
    pub tracks_assessed: usize,
    /// Tracks needing attention
    pub tracks_needing_attention: usize,
}
