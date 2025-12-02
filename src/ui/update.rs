//! Update handlers for application messages.

use iced::Task;
use smallvec::smallvec;
use std::path::PathBuf;

use crate::{db, library, metadata, organizer, enrichment, player, diagnostics};

use super::messages::Message;
use super::platform::get_user_music_folder;
use super::state::{AppState, EnrichmentState, LoadedState, OrganizeView, ActivePane, VisualizationMode};

/// Helper to load tracks from database
fn load_tracks_task(pool: sqlx::SqlitePool) -> Task<Message> {
    Task::perform(
        async move { db::get_all_tracks_with_metadata(&pool).await.map_err(|e| e.to_string()) },
        Message::TracksLoaded,
    )
}

/// Helper to pick a folder
fn pick_folder_task(on_pick: fn(Option<PathBuf>) -> Message) -> Task<Message> {
    Task::perform(
        async { rfd::AsyncFileDialog::new().pick_folder().await.map(|h| h.path().to_path_buf()) },
        on_pick,
    )
}

/// Helper to run diagnostics
fn run_diagnostics_task() -> Task<Message> {
    Task::perform(
        async {
            tokio::task::spawn_blocking(diagnostics::DiagnosticReport::generate)
                .await
                .expect("Diagnostics task failed")
        },
        Message::DiagnosticsComplete,
    )
}

/// Handle database initialization
pub fn handle_db_init(state: &mut AppState, result: Result<sqlx::SqlitePool, String>) -> Task<Message> {
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
                organize_destination: music_folder,
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
                diagnostics: None,
                diagnostics_loading: true,
            }));
            // Load tracks and run diagnostics in parallel
            Task::batch([
                load_tracks_task(pool),
                run_diagnostics_task(),
            ])
        }
        Err(e) => {
            *state = AppState::Error(e);
            Task::none()
        }
    }
}

/// Handle scan-related messages
pub fn handle_scan(s: &mut LoadedState, msg: &Message) -> Task<Message> {
    match msg {
        Message::ScanPressed => {
            s.is_scanning = true;
            s.scan_count = 0;
            s.status_message = "Scanning...".to_string();
            Task::none()
        }
        Message::ScanStopped => {
            s.is_scanning = false;
            s.status_message = "Scan stopped by user.".to_string();
            load_tracks_task(s.pool.clone())
        }
        Message::ScanFinished => {
            s.is_scanning = false;
            s.status_message = format!("Scan Complete. Processed {} files.", s.scan_count);
            load_tracks_task(s.pool.clone())
        }
        Message::ScanEventReceived(event) => {
            match event {
                library::ScanEvent::Processed(path) => {
                    s.scan_count += 1;
                    s.status_message = format!("Scanned {} files. Last: {:?}", s.scan_count, path.file_name().unwrap_or_default());
                }
                library::ScanEvent::Error(path, err) => {
                    s.status_message = format!("Error scanning {:?}: {}", path, err);
                }
            }
            Task::none()
        }
        _ => Task::none(),
    }
}

/// Handle organize-related messages
pub fn handle_organize(s: &mut LoadedState, msg: Message) -> Task<Message> {
    match msg {
        Message::OrganizeDestinationChanged(dest) => { s.organize_destination = PathBuf::from(dest); }
        Message::OrganizePatternChanged(pattern) => { s.organize_pattern = pattern; }
        Message::PickOrganizeDestination => return pick_folder_task(Message::OrganizeDestinationPicked),
        Message::OrganizeDestinationPicked(Some(path)) => { s.organize_destination = path; }
        Message::OrganizePreviewPressed => {
            s.organize_preview.clear();
            s.organize_view = OrganizeView::Preview;
            s.preview_loading = true;
            s.preview_scroll_offset = 0.0;
        }
        Message::OrganizePreviewBatch(batch) => { s.organize_preview.extend(batch); }
        Message::OrganizePreviewComplete => { s.preview_loading = false; }
        Message::OrganizeCancelPressed => {
            s.organize_view = OrganizeView::Input;
            s.organize_preview.clear();
            s.preview_loading = false;
        }
        Message::OrganizeConfirmPressed => return start_organize(s),
        Message::OrganizeFileComplete(result) => {
            s.organize_progress += 1;
            if let Err(e) = result { s.organize_errors.push(e); }
        }
        Message::OrganizeFinished => return finish_organize(s),
        _ => {}
    }
    Task::none()
}

/// Start the organize operation
fn start_organize(s: &mut LoadedState) -> Task<Message> {
    s.organize_view = OrganizeView::Organizing;
    s.organize_progress = 0;
    s.organize_total = s.organize_preview.len();
    s.organize_errors.clear();

    let pool = s.pool.clone();
    let pattern = s.organize_pattern.clone();
    let destination = s.organize_destination.clone();
    let previews = s.organize_preview.clone();

    Task::perform(
        async move {
            let mut undo_log = organizer::UndoLog { moves: vec![], timestamp: Some(chrono::Utc::now().to_rfc3339()) };
            let mut results = vec![];

            for preview in previews {
                let src = preview.source.clone();
                let pat = pattern.clone();
                let dest = destination.clone();

                let res = tokio::task::spawn_blocking(move || {
                    let meta = metadata::read(&src)?;
                    organizer::organize_track(&src, &meta, &pat, &dest).map(|p| (src, p))
                }).await;

                match res {
                    Ok(Ok((src, new_path))) => {
                        let path_str = new_path.to_string_lossy().to_string();
                        if let Err(e) = db::update_track_path(&pool, preview.track_id, &path_str).await {
                            results.push(Err(format!("DB error: {}", e)));
                        } else {
                            undo_log.moves.push(organizer::MoveRecord { source: src, destination: new_path, track_id: preview.track_id });
                            results.push(Ok(()));
                        }
                    }
                    Ok(Err(e)) => results.push(Err(format!("{}: {}", preview.source.display(), e))),
                    Err(e) => results.push(Err(format!("Task error: {}", e))),
                }
            }

            let log = undo_log;
            let _ = tokio::task::spawn_blocking(move || log.save()).await;
            results
        },
        |_| Message::OrganizeFinished,
    )
}

/// Finish the organize operation
fn finish_organize(s: &mut LoadedState) -> Task<Message> {
    let errors = s.organize_errors.len();
    let success = s.organize_total - errors;
    s.status_message = if errors == 0 {
        format!("Organized {} files successfully.", success)
    } else {
        format!("Organized {} of {} files. {} errors.", success, s.organize_total, errors)
    };
    s.organize_view = OrganizeView::Input;
    s.organize_preview.clear();
    s.can_undo = organizer::UndoLog::has_undo();
    load_tracks_task(s.pool.clone())
}

/// Handle undo-related messages
pub fn handle_undo(s: &mut LoadedState, msg: Message) -> Task<Message> {
    match msg {
        Message::UndoPressed => {
            s.status_message = "Undoing last organize...".to_string();
            let pool = s.pool.clone();
            Task::perform(
                async move {
                    let log = tokio::task::spawn_blocking(organizer::UndoLog::load)
                        .await
                        .map_err(|e| format!("Task error: {}", e))?;

                    let Some(log) = log else { return Err("No undo history available".to_string()) };

                    let mut count = 0;
                    for rec in &log.moves {
                        let r = rec.clone();
                        if let Ok(Ok(())) = tokio::task::spawn_blocking(move || organizer::undo_move(&r)).await {
                            let _ = db::update_track_path(&pool, rec.track_id, &rec.source.to_string_lossy()).await;
                            count += 1;
                        }
                    }
                    let _ = tokio::task::spawn_blocking(organizer::UndoLog::clear).await;
                    Ok(count)
                },
                Message::UndoComplete,
            )
        }
        Message::UndoComplete(result) => {
            match result {
                Ok(n) => { s.status_message = format!("Undo complete. Restored {} files.", n); s.can_undo = false; }
                Err(e) => { s.status_message = format!("Undo failed: {}", e); }
            }
            load_tracks_task(s.pool.clone())
        }
        _ => Task::none(),
    }
}

/// Handle enrichment-related messages
pub fn handle_enrichment(s: &mut LoadedState, msg: Message) -> Task<Message> {
    match msg {
        Message::EnrichmentApiKeyChanged(key) => {
            s.enrichment.api_key = key;
        }
        Message::EnrichmentTrackSelected(idx) => {
            s.enrichment.selected_track = Some(idx);
            s.enrichment.last_result = None;
            s.enrichment.last_error = None;
        }
        Message::EnrichmentIdentifyPressed => {
            let Some(idx) = s.enrichment.selected_track else { return Task::none() };
            let Some(track) = s.tracks.get(idx) else { return Task::none() };
            
            if s.enrichment.api_key.is_empty() {
                s.enrichment.last_error = Some("API key required. Get one at acoustid.org".to_string());
                return Task::none();
            }
            
            if !s.enrichment.fpcalc_available {
                s.enrichment.last_error = Some("fpcalc not installed. Run 'check-tools' for help.".to_string());
                return Task::none();
            }
            
            s.enrichment.is_identifying = true;
            s.enrichment.last_result = None;
            s.enrichment.last_error = None;
            
            let path = PathBuf::from(&track.path);
            let api_key = s.enrichment.api_key.clone();
            
            return Task::perform(
                async move {
                    let config = enrichment::EnrichmentConfig {
                        acoustid_api_key: api_key,
                        min_confidence: 0.5,
                        use_musicbrainz: true,
                        ..Default::default()
                    };
                    let service = enrichment::EnrichmentService::new(config);
                    service.identify_track(&path).await.map_err(|e| e.to_string())
                },
                Message::EnrichmentIdentifyResult,
            );
        }
        Message::EnrichmentIdentifyResult(result) => {
            s.enrichment.is_identifying = false;
            match result {
                Ok(identification) => {
                    s.enrichment.last_result = Some(identification);
                    s.enrichment.last_error = None;
                }
                Err(e) => {
                    s.enrichment.last_result = None;
                    s.enrichment.last_error = Some(e);
                }
            }
        }
        Message::EnrichmentClearResult => {
            s.enrichment.last_result = None;
            s.enrichment.last_error = None;
        }
        Message::EnrichmentWriteTagsPressed => {
            let Some(idx) = s.enrichment.selected_track else { return Task::none() };
            let Some(track) = s.tracks.get(idx) else { return Task::none() };
            let Some(ref result) = s.enrichment.last_result else { return Task::none() };
            
            let path = PathBuf::from(&track.path);
            let identified = result.track.clone();
            
            return Task::perform(
                async move {
                    let options = metadata::WriteOptions2 {
                        only_fill_empty: false,  // Overwrite with enriched data
                        write_musicbrainz_ids: true,
                    };
                    tokio::task::spawn_blocking(move || {
                        metadata::write(&path, &identified, &options)
                            .map(|r| r.fields_updated)
                            .map_err(|e| e.to_string())
                    })
                    .await
                    .map_err(|e| e.to_string())?
                },
                Message::EnrichmentWriteTagsResult,
            );
        }
        Message::EnrichmentWriteTagsResult(result) => {
            match result {
                Ok(count) => {
                    s.status_message = format!("✓ Tags written ({} fields updated)", count);
                    // Reload tracks to show updated metadata
                    return load_tracks_task(s.pool.clone());
                }
                Err(e) => {
                    s.enrichment.last_error = Some(format!("Failed to write tags: {}", e));
                }
            }
        }
        _ => {}
    }
    Task::none()
}

/// Handle player-related messages
pub fn handle_player(s: &mut LoadedState, msg: Message) -> Task<Message> {
    // Ensure player is initialized
    s.ensure_player();
    
    let Some(ref mut player) = s.player else {
        s.status_message = "Audio output not available".to_string();
        return Task::none();
    };
    
    match msg {
        Message::PlayerPlay => {
            // If queue is empty and nothing playing, start random shuffle
            if player.queue_mut().is_empty() && s.player_state.current_track.is_none() {
                // Trigger random shuffle instead
                use rand::seq::SliceRandom;
                let mut rng = rand::rng();
                
                let mut indices: Vec<usize> = (0..s.tracks.len()).collect();
                indices.shuffle(&mut rng);
                let count = 25.min(indices.len());
                
                for &idx in indices.iter().take(count) {
                    if let Some(track) = s.tracks.get(idx) {
                        player.queue_file(PathBuf::from(&track.path));
                    }
                }
                
                if let Err(e) = player.skip_forward() {
                    s.status_message = format!("Play error: {}", e);
                    s.player_state = player.state();
                } else {
                    s.status_message = format!("Started shuffle with {} random tracks", count);
                    s.auto_queue_enabled = true;
                    // Get state then set Playing optimistically (async command may not have processed yet)
                    s.player_state = player.state();
                    s.player_state.status = player::PlaybackStatus::Playing;
                }
            } else {
                if let Err(e) = player.play() {
                    s.status_message = format!("Play error: {}", e);
                }
                s.player_state = player.state();
            }
        }
        Message::PlayerPause => {
            if let Err(e) = player.pause() {
                s.status_message = format!("Pause error: {}", e);
            }
            s.player_state = player.state(); // Update state immediately for UI
        }
        Message::PlayerToggle => {
            if let Err(e) = player.toggle() {
                s.status_message = format!("Toggle error: {}", e);
            }
            s.player_state = player.state(); // Update state immediately for UI
        }
        Message::PlayerStop => {
            if let Err(e) = player.stop() {
                s.status_message = format!("Stop error: {}", e);
            }
            s.player_state = player.state(); // Update state immediately for UI
        }
        Message::PlayerNext => {
            if let Err(e) = player.skip_forward() {
                s.status_message = format!("Next error: {}", e);
            }
            s.player_state = player.state(); // Update state immediately for UI
        }
        Message::PlayerPrevious => {
            if let Err(e) = player.previous() {
                s.status_message = format!("Previous error: {}", e);
            }
            s.player_state = player.state(); // Update state immediately for UI
        }
        Message::PlayerSeek(pos) => {
            if let Err(e) = player.seek(pos) {
                s.status_message = format!("Seek error: {}", e);
            }
        }
        Message::PlayerVolumeChanged(vol) => {
            player.set_volume(vol);
        }
        Message::PlayerPlayTrack(idx) => {
            if let Some(track) = s.tracks.get(idx) {
                let path = PathBuf::from(&track.path);
                let artist = &track.artist_name;
                
                // Queue remaining tracks from the same artist (simulates album context)
                let mut queued_count = 0;
                for (i, t) in s.tracks.iter().enumerate() {
                    if i > idx && t.artist_name == *artist && queued_count < 20 {
                        player.queue_file(PathBuf::from(&t.path));
                        queued_count += 1;
                    }
                }
                
                if let Err(e) = player.play_file(path) {
                    s.status_message = format!("Failed to play: {}", e);
                } else {
                    s.status_message = format!("Playing: {} (+{} queued)", track.title, queued_count);
                    s.player_state = player.state();
                    s.auto_queue_enabled = true; // Enable auto-queue
                }
            }
        }
        Message::PlayerQueueTrack(idx) => {
            if let Some(track) = s.tracks.get(idx) {
                let path = PathBuf::from(&track.path);
                player.queue_file(path);
                s.status_message = format!("Queued: {}", track.title);
            }
        }
        Message::PlayerShuffleRandom => {
            use rand::seq::SliceRandom;
            let mut rng = rand::rng();
            
            // Pick 25 random tracks
            let mut indices: Vec<usize> = (0..s.tracks.len()).collect();
            indices.shuffle(&mut rng);
            let count = 25.min(indices.len());
            
            // Clear and queue
            player.queue_mut().clear();
            for &idx in indices.iter().take(count) {
                if let Some(track) = s.tracks.get(idx) {
                    player.queue_file(PathBuf::from(&track.path));
                }
            }
            
            // Start playing first track
            if let Err(e) = player.skip_forward() {
                s.status_message = format!("Shuffle error: {}", e);
            } else {
                s.status_message = format!("Shuffled {} random tracks", count);
                s.player_state = player.state();
                s.auto_queue_enabled = true;
            }
        }
        Message::PlayerTick => {
            // Update player state snapshot
            s.player_state = player.state();
            
            // Auto-queue: if queue is running low (< 5 remaining), add more random tracks
            if s.auto_queue_enabled && !s.tracks.is_empty() {
                let queue = player.queue();
                let remaining = queue.remaining_count();
                
                if remaining < 5 {
                    use rand::seq::SliceRandom;
                    let mut rng = rand::rng();
                    
                    // Add 8 more random tracks
                    let mut indices: Vec<usize> = (0..s.tracks.len()).collect();
                    indices.shuffle(&mut rng);
                    let add_count = 8.min(indices.len());
                    
                    for &idx in indices.iter().take(add_count) {
                        if let Some(track) = s.tracks.get(idx) {
                            player.queue_file(PathBuf::from(&track.path));
                        }
                    }
                    s.status_message = format!("Auto-queued {} more tracks", add_count);
                }
            }
        }
        Message::PlayerVisualizationTick => {
            // Get latest visualization data
            if let Some(viz) = player.visualization() {
                s.visualization = viz;
            }
        }
        Message::PlayerVisualizationModeChanged(mode) => {
            s.visualization_mode = mode;
        }
        Message::PlayerSelectDevice(device_name) => {
            // Note: Switching devices at runtime requires reinitializing the audio output
            // For now, just track the selection - a restart will pick up the preference
            s.current_audio_device = device_name.clone();
            s.status_message = format!("Audio device: {} (restart to apply)", device_name);
        }
        _ => {}
    }
    Task::none()
}

/// Handle diagnostics-related messages
pub fn handle_diagnostics(s: &mut LoadedState, msg: Message) -> Task<Message> {
    match msg {
        Message::DiagnosticsRunPressed => {
            s.diagnostics_loading = true;
            s.diagnostics = None;
            
            return Task::perform(
                async {
                    tokio::task::spawn_blocking(diagnostics::DiagnosticReport::generate)
                        .await
                        .expect("Diagnostics task failed")
                },
                Message::DiagnosticsComplete,
            );
        }
        Message::DiagnosticsComplete(report) => {
            s.diagnostics_loading = false;
            s.diagnostics = Some(report);
        }
        _ => {}
    }
    Task::none()
}
