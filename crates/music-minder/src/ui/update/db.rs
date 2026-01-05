//! Database initialization handler.

use iced::Task;
use smallvec::smallvec;
use std::time::Instant;

use crate::{config, diagnostics, enrichment, health, organizer, player};

use super::super::messages::Message;
use super::super::platform::get_user_music_folder;
use super::super::state::{
    ActivePane, AppState, EnrichmentPaneState, EnrichmentState, FocusedList, GardenerState,
    LoadedState, OrganizeView, SortColumn, VisualizationMode, WatcherState,
};
use super::load_tracks_initial_task;

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

/// Helper to enumerate audio devices in the background (deferred startup)
fn enumerate_audio_devices_task() -> Task<Message> {
    Task::perform(
        async {
            tokio::task::spawn_blocking(|| {
                tracing::debug!("Enumerating audio devices...");
                player::list_audio_devices()
            })
            .await
            .unwrap_or_default()
        },
        Message::AudioDevicesEnumerated,
    )
}

/// Handle database initialization
pub fn handle_db_init(
    state: &mut AppState,
    result: Result<sqlx::SqlitePool, String>,
) -> Task<Message> {
    match result {
        Ok(pool) => {
            let startup_start = Instant::now();
            tracing::debug!("handle_db_init() started");

            // Load config from disk (or defaults)
            let cfg = config::load();

            let music_folder = get_user_music_folder();
            let fpcalc_available = enrichment::fingerprint::is_fpcalc_available();

            // API key priority: config file > environment variable > default
            let api_key = cfg.credentials.acoustid_api_key.clone().unwrap_or_else(|| {
                std::env::var("ACOUSTID_API_KEY")
                    .unwrap_or_else(|_| enrichment::DEFAULT_ACOUSTID_API_KEY.to_string())
            });

            // OPTIMIZATION: Defer player creation to first play command
            // This avoids audio device enumeration at startup (can take 50-200ms)
            // Player will be lazily initialized via ensure_player() when user interacts
            let player_instance = None;
            let player_state = player::PlayerState::default();

            // OPTIMIZATION: Defer audio device enumeration to background task
            // Use empty list initially - devices will be populated when task completes
            let audio_devices = vec![];
            let current_audio_device = cfg.audio.output_device.clone();

            // Parse visualization mode from config
            let visualization_mode = match cfg.audio.visualization_mode.as_str() {
                "waveform" => VisualizationMode::Waveform,
                "vu_meter" => VisualizationMode::VuMeter,
                "off" => VisualizationMode::Off,
                _ => VisualizationMode::Spectrum,
            };

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
                tracks_total: None,
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
                    api_key: api_key.clone(),
                    fpcalc_available,
                    ..Default::default()
                },
                enrichment_pane: EnrichmentPaneState {
                    api_key,
                    fpcalc_available,
                    fill_only: true, // Default to safer option
                    fetch_cover_art: true,
                    ..Default::default()
                },
                player: player_instance,
                player_state,
                file_metadata: None,
                visualization: player::SpectrumData::default(),
                visualization_mode,
                auto_queue_enabled: cfg.library.auto_queue,
                audio_devices,
                current_audio_device,
                seek_preview: None,
                media_controls,
                cover_art: Default::default(),
                diagnostics: None,
                diagnostics_loading: true,
                diagnostics_started_tick: 0, // Starting at tick 0
                diagnostics_pending: None,
                diagnostics_expanded: std::collections::HashSet::new(),
                // Request high resolution timer for better audio scheduling
                #[cfg(windows)]
                high_res_timer: diagnostics::HighResolutionTimer::request(),
                animation_tick: 0,
                watcher_state: WatcherState {
                    active: true, // Start watching by default
                    watch_paths: vec![music_folder],
                    ..Default::default()
                },
                // Start the quality gardener
                gardener_state: {
                    let gardener = health::QualityGardener::new(pool.clone());
                    let command_tx = gardener.command_sender();
                    // Start the gardener in the background
                    let _handle = gardener.start();
                    tracing::info!("Quality gardener started");
                    GardenerState {
                        active: true,
                        command_tx: Some(command_tx),
                        ..Default::default()
                    }
                },
                // Search and filter state
                search_query: String::new(),
                filtered_indices: vec![],
                sort_column: SortColumn::Title,
                sort_ascending: true,
                filter_format: None,
                filter_lossless: None,
                // Sidebar state
                sidebar_collapsed: cfg.appearance.sidebar_collapsed,
                // Organize section collapsed state
                organize_collapsed: true, // Collapsed by default per design spec
                // Selection and focus state for keyboard navigation
                focused_list: FocusedList::Library,
                library_selection: None,
                queue_selection: None,
                // Queue drag-and-drop state
                queue_drag: Default::default(),
                // Easter egg state - random starting point
                easter_egg_index: (std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as usize)
                    % 8, // 8 easter eggs
                easter_egg_clicks: 0,
                // Track detail modal state
                track_detail: Default::default(),
                // Toast notifications
                toasts: Default::default(),
            }));

            tracing::debug!(
                "LoadedState created in {:.1}ms",
                startup_start.elapsed().as_secs_f64() * 1000.0
            );

            // Progressive loading: load first batch quickly, then rest in background
            // Also run diagnostics and enumerate audio devices in parallel
            Task::batch([
                load_tracks_initial_task(pool),
                run_diagnostics_task(),
                enumerate_audio_devices_task(),
            ])
        }
        Err(e) => {
            *state = AppState::Error(e);
            Task::none()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that enumerate_audio_devices_task creates a valid Task.
    /// This is a compile-time verification that the async operation is properly structured.
    #[test]
    fn enumerate_audio_devices_task_compiles() {
        // This test verifies the task creation doesn't panic and returns a valid type
        let _task: Task<Message> = enumerate_audio_devices_task();
    }

    /// Test that run_diagnostics_task creates a valid Task.
    #[test]
    fn run_diagnostics_task_compiles() {
        let _task: Task<Message> = run_diagnostics_task();
    }

    /// Verify player::PlayerState defaults are sane for startup.
    #[test]
    fn player_state_default_is_stopped() {
        let state = player::PlayerState::default();
        assert_eq!(state.status, player::PlaybackStatus::Stopped);
        assert_eq!(state.volume, 1.0);
        assert!(state.position.is_zero());
        assert!(state.duration.is_zero());
    }

    /// Test config loading doesn't panic with missing file.
    #[test]
    fn config_load_with_missing_file_returns_defaults() {
        // config::load() should return defaults, not panic
        let cfg = config::load();
        // Just verify it returns something - defaults are fine
        let _ = cfg.audio.output_device;
    }
}
