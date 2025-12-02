//! UI module for Music Minder.

mod canvas;
pub mod icons;
mod messages;
mod platform;
mod state;
mod streams;
mod update;
mod views;

use iced::widget::{container, text};
use iced::{Element, Length, Subscription, Task, time};
use std::path::PathBuf;
use std::time::Duration;

use crate::player::PlaybackStatus;
pub use messages::Message;
use state::AppState;

pub struct MusicMinder {
    state: AppState,
}

impl MusicMinder {
    pub fn new() -> (Self, Task<Message>) {
        let init_db = Task::perform(
            async {
                crate::db::init_db("sqlite:music_minder.db")
                    .await
                    .map_err(|e| e.to_string())
            },
            Message::DbInitialized,
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

        // Player state update (every 100ms when playing)
        if s.player_state.status == PlaybackStatus::Playing {
            subscriptions
                .push(time::every(Duration::from_millis(100)).map(|_| Message::PlayerTick));

            // Visualization update (every 33ms = ~30fps) - always run when playing
            subscriptions.push(
                time::every(Duration::from_millis(33)).map(|_| Message::PlayerVisualizationTick),
            );
        }

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
        // Handle messages that work regardless of state
        match &message {
            Message::DbInitialized(result) => {
                return update::handle_db_init(&mut self.state, result.clone());
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

            // Tracks loaded
            Message::TracksLoaded(Ok(tracks)) => {
                s.tracks = tracks.clone();
                s.tracks_loading = false;
                s.status_message = format!("{} tracks loaded.", s.tracks.len());
            }
            Message::TracksLoaded(Err(e)) => {
                s.tracks_loading = false;
                s.status_message = format!("Error loading tracks: {}", e);
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

            // Enrichment messages
            Message::EnrichmentApiKeyChanged(_)
            | Message::EnrichmentTrackSelected(_)
            | Message::EnrichmentIdentifyPressed
            | Message::EnrichmentIdentifyResult(_)
            | Message::EnrichmentClearResult
            | Message::EnrichmentWriteTagsPressed
            | Message::EnrichmentWriteTagsResult(_) => {
                return update::handle_enrichment(s, message);
            }

            // Player messages
            Message::PlayerPlay
            | Message::PlayerPause
            | Message::PlayerToggle
            | Message::PlayerStop
            | Message::PlayerNext
            | Message::PlayerPrevious
            | Message::PlayerSeek(_)
            | Message::PlayerVolumeChanged(_)
            | Message::PlayerPlayTrack(_)
            | Message::PlayerQueueTrack(_)
            | Message::PlayerTick
            | Message::PlayerVisualizationTick
            | Message::PlayerVisualizationModeChanged(_) => {
                return update::handle_player(s, message);
            }

            // Diagnostics messages
            Message::DiagnosticsRunPressed | Message::DiagnosticsComplete(_) => {
                return update::handle_diagnostics(s, message);
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
