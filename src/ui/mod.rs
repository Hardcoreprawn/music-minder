//! UI module for Music Minder.

mod canvas;
pub mod icons;
mod messages;
mod platform;
mod state;
mod streams;
pub mod theme;
mod update;
mod views;

use iced::widget::{container, text};
use iced::{Element, Length, Subscription, Task, keyboard, time};
use std::path::PathBuf;
use std::time::Duration;

pub use messages::Message;
use state::AppState;

pub struct MusicMinder {
    state: AppState,
}

impl MusicMinder {
    pub fn new() -> (Self, Task<Message>) {
        use std::time::Instant;

        let ui_init_start = Instant::now();
        tracing::debug!("UI::new() started");

        let init_db = Task::perform(
            async {
                let db_start = Instant::now();
                let result = crate::db::init_db("sqlite:music_minder.db")
                    .await
                    .map_err(|e| e.to_string());
                tracing::info!(
                    "Database init completed in {:.1}ms",
                    db_start.elapsed().as_secs_f64() * 1000.0
                );
                result
            },
            Message::DbInitialized,
        );

        tracing::debug!(
            "UI::new() task created in {:.1}ms",
            ui_init_start.elapsed().as_secs_f64() * 1000.0
        );

        (
            Self {
                state: AppState::Loading,
            },
            init_db,
        )
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let AppState::Loaded(s) = &self.state else {
            return Subscription::none();
        };

        let mut subscriptions = Vec::new();

        // Scan subscription
        if s.is_scanning {
            subscriptions.push(Subscription::run_with_id(
                "scan-library",
                streams::scan_stream(s.pool.clone(), s.scan_path.clone()),
            ));
        }

        // Preview subscription
        if s.preview_loading {
            subscriptions.push(Subscription::run_with_id(
                "preview-organize",
                streams::preview_stream(
                    s.pool.clone(),
                    s.organize_pattern.clone(),
                    s.organize_destination.clone(),
                ),
            ));
        }

        // Background file watcher - uses async channel to avoid blocking the runtime
        if s.watcher_state.active && !s.watcher_state.watch_paths.is_empty() {
            subscriptions.push(Subscription::run_with_id(
                "file-watcher",
                streams::watcher_stream(s.watcher_state.watch_paths.clone()),
            ));
        }

        // Player event polling and UI animation tick
        // Uses time::every() instead of window::frames() because:
        // 1. window::frames() fires at monitor refresh rate and caused channel overflow
        // 2. This caused "TrySendError { kind: Full }" and dropped ticks
        // 60fps (16ms) is a good balance for smooth animations without overwhelming.
        // Audio playback is on a separate thread and unaffected by this rate.
        subscriptions.push(time::every(Duration::from_millis(16)).map(|_| Message::PlayerTick));

        // Keyboard shortcuts - global within the app
        subscriptions.push(keyboard::on_key_press(|key, modifiers| {
            Some(Message::KeyPressed(key, modifiers))
        }));

        Subscription::batch(subscriptions)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let content: Element<Message> = match &self.state {
            AppState::Loading => text("Loading database...").size(30).into(),
            AppState::Loaded(s) => views::loaded_view(s),
            AppState::Error(e) => text(format!("Error: {}", e))
                .size(30)
                .color([0.9, 0.0, 0.0])
                .into(),
        };
        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20)
            .into()
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        // Debug: log every message type at top level
        let is_tick = matches!(
            message,
            Message::PlayerTick | Message::PlayerVisualizationTick
        );
        if !is_tick {
            tracing::trace!(target: "ui::update", message = ?message, "Update received");
        }

        // Handle messages that work regardless of state
        match &message {
            Message::DbInitialized(result) => {
                return update::handle_db_init(&mut self.state, result.clone());
            }
            Message::AudioDevicesEnumerated(devices) => {
                // Update audio devices if we're loaded
                if let AppState::Loaded(s) = &mut self.state {
                    tracing::debug!("Audio devices enumerated: {:?}", devices);
                    s.audio_devices = devices.clone();
                }
                return Task::none();
            }
            Message::PickPath => return pick_folder(Message::PathPicked),
            Message::FontLoaded => return Task::none(), // Font loaded successfully
            _ => {}
        }

        // Handle messages that require loaded state
        let AppState::Loaded(s) = &mut self.state else {
            return Task::none();
        };

        match &message {
            // Navigation
            Message::SwitchPane(pane) => {
                s.active_pane = *pane;
            }
            Message::ToggleSidebar => {
                s.sidebar_collapsed = !s.sidebar_collapsed;
            }
            Message::ToggleOrganizeSection => {
                s.organize_collapsed = !s.organize_collapsed;
            }
            Message::PlaceholderClicked => {
                s.easter_egg_clicks += 1;
                // After 10 clicks, cycle to the next easter egg
                if s.easter_egg_clicks >= 10 {
                    s.easter_egg_index = (s.easter_egg_index + 1) % 8;
                    s.easter_egg_clicks = 0;
                }
            }

            // Scroll updates
            Message::ScrollChanged(v) => {
                s.scroll_offset = v.absolute_offset().y;
                s.viewport_height = v.bounds().height;
            }
            Message::PreviewScrollChanged(v) => {
                s.preview_scroll_offset = v.absolute_offset().y;
                s.preview_viewport_height = v.bounds().height;
            }

            // Path updates
            Message::PathChanged(p) => {
                s.scan_path = PathBuf::from(p);
            }
            Message::PathPicked(Some(p)) => {
                s.scan_path = p.clone();
            }

            // Tracks loaded (legacy - full load)
            Message::TracksLoaded(Ok(tracks)) => {
                s.tracks = tracks.clone();
                s.tracks_loading = false;
                s.tracks_total = Some(s.tracks.len() as i64);
                s.status_message = format!("{} tracks loaded.", s.tracks.len());
            }
            Message::TracksLoaded(Err(e)) => {
                s.tracks_loading = false;
                s.status_message = format!("Error loading tracks: {}", e);
            }

            // Progressive loading: initial batch
            Message::TracksLoadedInitial(Ok((tracks, total))) => {
                s.tracks = tracks.clone();
                s.tracks_total = Some(*total);
                let loaded = s.tracks.len();

                if loaded as i64 >= *total {
                    // All tracks fit in initial batch
                    s.tracks_loading = false;
                    s.status_message = format!("{} tracks loaded.", loaded);
                    return Task::none();
                } else {
                    // More tracks to load - update status and kick off remaining load
                    s.status_message = format!("Loaded {} of {} tracks...", loaded, total);
                    return update::load_tracks_remaining_task(
                        s.pool.clone(),
                        loaded as i64,
                        *total,
                    );
                }
            }
            Message::TracksLoadedInitial(Err(e)) => {
                s.tracks_loading = false;
                s.status_message = format!("Error loading tracks: {}", e);
            }

            // Progressive loading: remaining tracks
            Message::TracksLoadedMore(Ok(tracks)) => {
                s.tracks.extend(tracks.iter().cloned());
                s.tracks_loading = false;
                s.status_message = format!("{} tracks loaded.", s.tracks.len());
            }
            Message::TracksLoadedMore(Err(e)) => {
                // Keep partial results, just log error
                s.tracks_loading = false;
                tracing::error!("Error loading remaining tracks: {}", e);
                s.status_message = format!("{} tracks loaded (some failed).", s.tracks.len());
            }

            // Scan messages
            Message::ScanPressed
            | Message::ScanStopped
            | Message::ScanFinished
            | Message::ScanEventReceived(_) => {
                return update::handle_scan(s, &message);
            }

            // Organize messages
            Message::OrganizeDestinationChanged(_)
            | Message::OrganizePatternChanged(_)
            | Message::PickOrganizeDestination
            | Message::OrganizeDestinationPicked(_)
            | Message::OrganizePreviewPressed
            | Message::OrganizePreviewBatch(_)
            | Message::OrganizePreviewComplete
            | Message::OrganizeCancelPressed
            | Message::OrganizeConfirmPressed
            | Message::OrganizeFileComplete(_)
            | Message::OrganizeFinished => {
                return update::handle_organize(s, message);
            }

            // Undo messages
            Message::UndoPressed | Message::UndoComplete(_) => {
                return update::handle_undo(s, message);
            }

            // Enrichment messages (single track - Settings pane)
            Message::EnrichmentApiKeyChanged(_)
            | Message::EnrichmentApiKeySave
            | Message::EnrichmentApiKeySaved
            | Message::EnrichmentTrackSelected(_)
            | Message::EnrichmentIdentifyPressed
            | Message::EnrichmentIdentifyResult(_)
            | Message::EnrichmentClearResult
            | Message::EnrichmentWriteTagsPressed
            | Message::EnrichmentWriteTagsResult(_) => {
                return update::handle_enrichment(s, message);
            }

            // Enrich Pane messages (batch operations)
            Message::EnrichAddFromLibrary
            | Message::EnrichAddTracks(_)
            | Message::EnrichRemoveTrack(_)
            | Message::EnrichClearTracks
            | Message::EnrichTrackChecked(_, _)
            | Message::EnrichFillOnlyToggled(_)
            | Message::EnrichFetchCoverArtToggled(_)
            | Message::EnrichBatchIdentify
            | Message::EnrichBatchIdentifyResult(_, _)
            | Message::EnrichBatchIdentifyWithAlts(_, _)
            | Message::EnrichBatchComplete
            | Message::EnrichReviewResult(_)
            | Message::EnrichToggleAlternatives(_)
            | Message::EnrichSelectAlternative(_, _)
            | Message::EnrichWriteResult(_)
            | Message::EnrichWriteAllConfirmed
            | Message::EnrichExportReport => {
                return update::handle_enrich_pane(s, message);
            }

            // Player messages
            Message::PlayerPlay
            | Message::PlayerPause
            | Message::PlayerToggle
            | Message::PlayerStop
            | Message::PlayerNext
            | Message::PlayerPrevious
            | Message::PlayerSeekPreview(_)
            | Message::PlayerSeekRelease
            | Message::PlayerVolumeChanged(_)
            | Message::PlayerPlayTrack(_)
            | Message::PlayerQueueTrack(_)
            | Message::PlayerTick
            | Message::PlayerShuffleRandom
            | Message::PlayerSelectDevice(_)
            | Message::PlayerVisualizationTick
            | Message::PlayerVisualizationModeChanged(_)
            | Message::PlayerEvent(_)
            | Message::MediaControlCommand(_)
            | Message::MediaControlPoll
            | Message::QueueJumpTo(_)
            | Message::QueueRemove(_)
            | Message::QueueClear
            | Message::QueueToggleShuffle
            | Message::QueueCycleRepeat => {
                // Note: MediaControlPoll is now handled in PlayerTick for simplicity,
                // but we keep it routed here as a fallback
                return update::handle_player(s, message);
            }

            // Diagnostics messages
            Message::DiagnosticsRunPressed
            | Message::DiagnosticsComplete(_)
            | Message::DiagnosticsToggleCheck(_)
            | Message::CoverArtResolved(_, _) => {
                return update::handle_diagnostics(s, message);
            }

            // File watcher messages
            Message::WatcherStarted
            | Message::WatcherStopped
            | Message::WatcherEvent(_)
            | Message::LibraryFileChanged(_)
            | Message::RescanLibrary => {
                return update::handle_watcher(s, message);
            }

            // Search and filter messages
            Message::SearchQueryChanged(_)
            | Message::SortByColumn(_)
            | Message::FilterByFormat(_)
            | Message::FilterByLossless(_)
            | Message::ClearFilters => {
                return update::handle_search_filter(s, message);
            }

            // Selection / keyboard navigation messages
            Message::LibrarySelectPrevious
            | Message::LibrarySelectNext
            | Message::LibrarySelectIndex(_)
            | Message::QueueSelectPrevious
            | Message::QueueSelectNext
            | Message::QueueSelectIndex(_)
            | Message::QueueMoveUp
            | Message::QueueMoveDown
            | Message::QueueDragStart { .. }
            | Message::QueueDragMove { .. }
            | Message::QueueDragEnd
            | Message::QueueDragCancel
            | Message::PlaySelected
            | Message::RemoveSelectedFromQueue => {
                return update::handle_selection(s, message);
            }

            // Track detail messages
            Message::TrackDetailOpen(_)
            | Message::TrackDetailClose
            | Message::TrackDetailIdentify
            | Message::TrackDetailIdentifyResult(_)
            | Message::TrackDetailWriteTags
            | Message::TrackDetailWriteResult(_)
            | Message::TrackDetailRefresh
            | Message::TrackDetailRefreshed(_) => {
                return update::handle_track_detail(s, message);
            }

            // Toast notification messages
            Message::ToastDismiss(id) => {
                s.toasts.remove(*id);
            }
            Message::ToastExpireTick => {
                s.toasts.remove_expired();
            }

            // Keyboard shortcuts
            Message::KeyPressed(key, modifiers) => {
                return update::handle_keyboard(s, key.clone(), *modifiers);
            }

            _ => {}
        }
        Task::none()
    }
}

fn pick_folder(on_pick: fn(Option<PathBuf>) -> Message) -> Task<Message> {
    Task::perform(
        async {
            rfd::AsyncFileDialog::new()
                .pick_folder()
                .await
                .map(|h| h.path().to_path_buf())
        },
        on_pick,
    )
}
