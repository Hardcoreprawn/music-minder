//! Database initialization handler.

use iced::Task;
use smallvec::smallvec;

use crate::{diagnostics, enrichment, organizer, player};

use super::super::messages::Message;
use super::super::platform::get_user_music_folder;
use super::super::state::{
    ActivePane, AppState, EnrichmentState, LoadedState, OrganizeView, VisualizationMode,
    WatcherState,
};
use super::load_tracks_task;

/// Helper to run diagnostics
fn run_diagnostics_task() -> Task<Message> {
    Task::perform(
        async {
            match tokio::task::spawn_blocking(diagnostics::DiagnosticReport::generate).await {
                Ok(report) => report,
                Err(e) => {
                    tracing::error!("Diagnostics task panicked: {}", e);
                    diagnostics::DiagnosticReport::default()
                }
            }
        },
        Message::DiagnosticsComplete,
    )
}

/// Handle database initialization
pub fn handle_db_init(
    state: &mut AppState,
    result: Result<sqlx::SqlitePool, String>,
) -> Task<Message> {
    match result {
        Ok(pool) => {
            let music_folder = get_user_music_folder();
            let fpcalc_available = enrichment::fingerprint::is_fpcalc_available();
            let api_key = std::env::var("ACOUSTID_API_KEY").unwrap_or_default();

            // Try to initialize player
            let player_instance = player::Player::new();
            let player_state = player::PlayerState::default();

            // Get audio device info
            let audio_devices = player::list_audio_devices();
            let current_audio_device = player::current_audio_device();

            // Initialize OS media controls (SMTC on Windows, MPRIS on Linux)
            let media_controls = player::MediaControlsHandle::new();
            if media_controls.is_some() {
                tracing::info!("OS media controls initialized");
            } else {
                tracing::warn!("OS media controls not available");
            }

            *state = AppState::Loaded(Box::new(LoadedState {
                pool: pool.clone(),
                active_pane: ActivePane::Library,
                scan_path: music_folder.clone(),
                is_scanning: false,
                tracks: vec![],
                tracks_loading: true,
                status_message: "Loading library...".to_string(),
                scan_count: 0,
                scroll_offset: 0.0,
                viewport_height: 0.0,
                preview_scroll_offset: 0.0,
                preview_viewport_height: 0.0,
                organize_destination: music_folder.clone(),
                organize_pattern: "{Artist}/{Album}/{TrackNum} - {Title}.{ext}".to_string(),
                organize_view: OrganizeView::default(),
                organize_preview: vec![],
                organize_progress: 0,
                organize_total: 0,
                organize_errors: smallvec![],
                can_undo: organizer::UndoLog::has_undo(),
                preview_loading: false,
                enrichment: EnrichmentState {
                    api_key,
                    fpcalc_available,
                    ..Default::default()
                },
                player: player_instance,
                player_state,
                visualization: player::SpectrumData::default(),
                visualization_mode: VisualizationMode::Spectrum,
                auto_queue_enabled: true,
                audio_devices,
                current_audio_device,
                media_controls,
                cover_art: Default::default(),
                diagnostics: None,
                diagnostics_loading: true,
                watcher_state: WatcherState {
                    active: true, // Start watching by default
                    watch_paths: vec![music_folder],
                    ..Default::default()
                },
            }));
            // Load tracks and run diagnostics in parallel
            Task::batch([load_tracks_task(pool), run_diagnostics_task()])
        }
        Err(e) => {
            *state = AppState::Error(e);
            Task::none()
        }
    }
}
