//! Message types for the Music Minder UI.

use super::state::{ActivePane, VisualizationMode};
use crate::{db, diagnostics, enrichment, library, organizer};
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
    PlayerSeek(f32),
    PlayerVolumeChanged(f32),
    PlayerPlayTrack(usize),     // Play track at index from library
    PlayerQueueTrack(usize),    // Add track to queue
    PlayerShuffleRandom,        // Shuffle 20-30 random tracks
    PlayerSelectDevice(String), // Switch audio output device
    PlayerTick,                 // Timer tick for updating UI
    PlayerVisualizationTick,    // Fast tick for visualization
    PlayerVisualizationModeChanged(VisualizationMode),

    // Diagnostics messages
    DiagnosticsRunPressed,
    DiagnosticsComplete(diagnostics::DiagnosticReport),
}
