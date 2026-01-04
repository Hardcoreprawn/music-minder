//! Diagnostics and cover art handlers.

use iced::Task;

use crate::diagnostics;

use super::super::messages::Message;
use super::super::state::LoadedState;

/// Handle diagnostics-related messages
pub fn handle_diagnostics(s: &mut LoadedState, msg: Message) -> Task<Message> {
    match msg {
        Message::DiagnosticsRunPressed => {
            s.diagnostics_loading = true;
            s.diagnostics = None;
            // Record when diagnostics started for animation timing
            s.diagnostics_started_tick = s.animation_tick;

            return Task::perform(
                async {
                    match tokio::task::spawn_blocking(diagnostics::DiagnosticReport::generate).await
                    {
                        Ok(report) => report,
                        Err(e) => {
                            tracing::error!("Diagnostics task panicked: {}", e);
                            diagnostics::DiagnosticReport::default()
                        }
                    }
                },
                Message::DiagnosticsComplete,
            );
        }
        Message::DiagnosticsComplete(report) => {
            // Store in pending - will be revealed when animation completes
            s.diagnostics_pending = Some(report);
        }
        Message::DiagnosticsToggleCheck(name) => {
            // Toggle expanded state for this check
            if s.diagnostics_expanded.contains(&name) {
                s.diagnostics_expanded.remove(&name);
            } else {
                s.diagnostics_expanded.insert(name);
            }
        }
        Message::CoverArtResolved(path, result) => {
            // Only update if this is still the current track
            if s.cover_art.for_track.as_ref() == Some(&path) {
                s.cover_art.loading = false;
                match result {
                    Ok(cover) => {
                        s.cover_art.current = Some(cover);
                        s.cover_art.error = None;
                    }
                    Err(e) => {
                        s.cover_art.current = None;
                        s.cover_art.error = Some(e);
                    }
                }
            }
        }
        _ => {}
    }
    Task::none()
}
