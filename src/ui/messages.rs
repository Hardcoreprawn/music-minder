//! Message types for the Music Minder UI.

use super::state::{ActivePane, LoadedCoverArt, VisualizationMode};
use crate::{db, diagnostics, enrichment, library, organizer, player, scanner};
use iced::widget::scrollable::Viewport;
use sqlx::SqlitePool;
use std::path::PathBuf;

/// All possible messages that can be sent in the application
#[derive(Debug, Clone)]
pub enum Message {
    // Initialization
    DbInitialized(Result<SqlitePool, String>),
    FontLoaded,

    // Navigation
    SwitchPane(ActivePane),

    // Scan messages
    PathChanged(String),
    PickPath,
    PathPicked(Option<PathBuf>),
    ScanPressed,
    ScanStopped,
    ScanEventReceived(library::ScanEvent),
    ScanFinished,
    TracksLoaded(Result<Vec<db::TrackWithMetadata>, String>),

    // Scroll messages
    ScrollChanged(Viewport),
    PreviewScrollChanged(Viewport),

    // Organize messages
    OrganizeDestinationChanged(String),
    OrganizePatternChanged(String),
    PickOrganizeDestination,
    OrganizeDestinationPicked(Option<PathBuf>),
    OrganizePreviewPressed,
    OrganizePreviewBatch(Vec<organizer::OrganizePreview>),
    OrganizePreviewComplete,
    OrganizeConfirmPressed,
    OrganizeFileComplete(Result<(i64, String), String>),
    OrganizeFinished,
    OrganizeCancelPressed,

    // Undo messages
    UndoPressed,
    UndoComplete(Result<usize, String>),

    // Enrichment messages
    EnrichmentApiKeyChanged(String),
    EnrichmentTrackSelected(usize),
    EnrichmentIdentifyPressed,
    EnrichmentIdentifyResult(Result<enrichment::TrackIdentification, String>),
    EnrichmentClearResult,
    EnrichmentWriteTagsPressed,
    EnrichmentWriteTagsResult(Result<usize, String>),

    // Player messages
    PlayerPlay,
    PlayerPause,
    PlayerToggle,
    PlayerStop,
    PlayerNext,
    PlayerPrevious,
    PlayerSeekPreview(f32),   // While dragging - updates display only
    PlayerSeekRelease,        // On release - performs actual seek using stored preview position
    PlayerVolumeChanged(f32),
    PlayerPlayTrack(usize),     // Play track at index from library
    PlayerQueueTrack(usize),    // Add track to queue
    PlayerShuffleRandom,        // Shuffle 20-30 random tracks
    PlayerSelectDevice(String), // Switch audio output device
    PlayerTick,                 // Timer tick for updating UI
    PlayerVisualizationTick,    // Fast tick for visualization
    PlayerVisualizationModeChanged(VisualizationMode),
    PlayerEvent(player::PlayerEvent), // Event from audio thread (state changed, track loaded, etc.)

    // OS Media control messages (from SMTC/MPRIS)
    MediaControlCommand(player::MediaControlCommand),
    MediaControlPoll, // Timer tick to poll for media control events

    // Diagnostics messages
    DiagnosticsRunPressed,
    DiagnosticsComplete(diagnostics::DiagnosticReport),

    // Cover art messages (background, non-blocking)
    CoverArtResolved(PathBuf, Result<LoadedCoverArt, String>),

    // Background scanner messages
    WatcherEvent(scanner::WatchEvent),
    WatcherStarted,
    WatcherStopped,
    LibraryFileChanged(PathBuf), // A file in the library changed, may need refresh
    RescanLibrary,               // Force a full library rescan
}
