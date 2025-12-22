//! Keyboard shortcut handling.
//!
//! Maps keyboard events to player and UI actions.
//! Start with in-app shortcuts, global hotkeys are a future feature.

use iced::Task;
use iced::keyboard::{self, key};

use super::super::messages::Message;
use super::super::state::{ActivePane, FocusedList, LoadedState};

/// Handle keyboard shortcuts.
///
/// Returns a Task if the key triggered an action, or Task::none() if unhandled.
pub fn handle_keyboard(
    s: &mut LoadedState,
    key: keyboard::Key,
    modifiers: keyboard::Modifiers,
) -> Task<Message> {
    // Don't handle keys when search box might be focused
    // (We'll refine this later with proper focus tracking)

    match key.as_ref() {
        // Space: Play/Pause toggle
        keyboard::Key::Named(key::Named::Space) => {
            if modifiers.is_empty() {
                tracing::debug!(target: "ui::keyboard", "Space pressed - toggling playback");
                return Task::done(Message::PlayerToggle);
            }
        }

        // Left Arrow: Previous track (or seek with Shift)
        keyboard::Key::Named(key::Named::ArrowLeft) => {
            if modifiers.shift() {
                // Shift+Left: Seek backward 5 seconds
                if s.player.is_some() {
                    let current_secs = s.player_state.position.as_secs_f32();
                    let duration_secs = s.player_state.duration.as_secs_f32();
                    if duration_secs > 0.0 {
                        let new_pos = (current_secs - 5.0).max(0.0);
                        let fraction = new_pos / duration_secs;
                        s.seek_preview = Some(fraction);
                        tracing::debug!(target: "ui::keyboard", "Shift+Left - seeking to {:.1}s", new_pos);
                        return Task::done(Message::PlayerSeekRelease);
                    }
                }
            } else if modifiers.is_empty() {
                tracing::debug!(target: "ui::keyboard", "Left pressed - previous track");
                return Task::done(Message::PlayerPrevious);
            }
        }

        // Right Arrow: Next track (or seek with Shift)
        keyboard::Key::Named(key::Named::ArrowRight) => {
            if modifiers.shift() {
                // Shift+Right: Seek forward 5 seconds
                if s.player.is_some() {
                    let current_secs = s.player_state.position.as_secs_f32();
                    let duration_secs = s.player_state.duration.as_secs_f32();
                    if duration_secs > 0.0 {
                        let new_pos = (current_secs + 5.0).min(duration_secs);
                        let fraction = new_pos / duration_secs;
                        s.seek_preview = Some(fraction);
                        tracing::debug!(target: "ui::keyboard", "Shift+Right - seeking to {:.1}s", new_pos);
                        return Task::done(Message::PlayerSeekRelease);
                    }
                }
            } else if modifiers.is_empty() {
                tracing::debug!(target: "ui::keyboard", "Right pressed - next track");
                return Task::done(Message::PlayerNext);
            }
        }

        // Up Arrow: Move selection up (or volume up with modifier)
        keyboard::Key::Named(key::Named::ArrowUp) => {
            if modifiers.alt() {
                // Alt+Up: Move queue item up (if queue focused) OR volume up
                if s.active_pane == ActivePane::NowPlaying && s.focused_list == FocusedList::Queue {
                    tracing::debug!(target: "ui::keyboard", "Alt+Up pressed - moving queue item up");
                    return Task::done(Message::QueueMoveUp);
                } else if let Some(player) = &s.player {
                    let current = player.volume();
                    let new_vol = (current + 0.05).min(1.1);
                    tracing::debug!(target: "ui::keyboard", "Alt+Up pressed - volume {:.0}%", new_vol * 100.0);
                    return Task::done(Message::PlayerVolumeChanged(new_vol));
                }
            } else if modifiers.is_empty() {
                // Up: Move selection up in focused list
                return match (s.active_pane, s.focused_list) {
                    (ActivePane::Library, FocusedList::Library) => {
                        tracing::debug!(target: "ui::keyboard", "Up pressed - library selection up");
                        Task::done(Message::LibrarySelectPrevious)
                    }
                    (ActivePane::NowPlaying, FocusedList::Queue) => {
                        tracing::debug!(target: "ui::keyboard", "Up pressed - queue selection up");
                        Task::done(Message::QueueSelectPrevious)
                    }
                    _ => {
                        // Fallback to volume for other panes
                        if let Some(player) = &s.player {
                            let current = player.volume();
                            let new_vol = (current + 0.05).min(1.1);
                            tracing::debug!(target: "ui::keyboard", "Up pressed - volume {:.0}%", new_vol * 100.0);
                            Task::done(Message::PlayerVolumeChanged(new_vol))
                        } else {
                            Task::none()
                        }
                    }
                };
            }
        }

        // Down Arrow: Move selection down (or volume down with modifier)
        keyboard::Key::Named(key::Named::ArrowDown) => {
            if modifiers.alt() {
                // Alt+Down: Move queue item down (if queue focused) OR volume down
                if s.active_pane == ActivePane::NowPlaying && s.focused_list == FocusedList::Queue {
                    tracing::debug!(target: "ui::keyboard", "Alt+Down pressed - moving queue item down");
                    return Task::done(Message::QueueMoveDown);
                } else if let Some(player) = &s.player {
                    let current = player.volume();
                    let new_vol = (current - 0.05).max(0.0);
                    tracing::debug!(target: "ui::keyboard", "Alt+Down pressed - volume {:.0}%", new_vol * 100.0);
                    return Task::done(Message::PlayerVolumeChanged(new_vol));
                }
            } else if modifiers.is_empty() {
                // Down: Move selection down in focused list
                return match (s.active_pane, s.focused_list) {
                    (ActivePane::Library, FocusedList::Library) => {
                        tracing::debug!(target: "ui::keyboard", "Down pressed - library selection down");
                        Task::done(Message::LibrarySelectNext)
                    }
                    (ActivePane::NowPlaying, FocusedList::Queue) => {
                        tracing::debug!(target: "ui::keyboard", "Down pressed - queue selection down");
                        Task::done(Message::QueueSelectNext)
                    }
                    _ => {
                        // Fallback to volume for other panes
                        if let Some(player) = &s.player {
                            let current = player.volume();
                            let new_vol = (current - 0.05).max(0.0);
                            tracing::debug!(target: "ui::keyboard", "Down pressed - volume {:.0}%", new_vol * 100.0);
                            Task::done(Message::PlayerVolumeChanged(new_vol))
                        } else {
                            Task::none()
                        }
                    }
                };
            }
        }

        // Enter: Play selected track
        keyboard::Key::Named(key::Named::Enter) => {
            if modifiers.is_empty() {
                tracing::debug!(target: "ui::keyboard", "Enter pressed - play selected");
                return Task::done(Message::PlaySelected);
            }
        }

        // Delete: Remove selected from queue
        keyboard::Key::Named(key::Named::Delete) => {
            if modifiers.is_empty() {
                tracing::debug!(target: "ui::keyboard", "Delete pressed - remove from queue");
                return Task::done(Message::RemoveSelectedFromQueue);
            }
        }

        // Escape: Clear search / close panels
        keyboard::Key::Named(key::Named::Escape) => {
            if modifiers.is_empty() && !s.search_query.is_empty() {
                tracing::debug!(target: "ui::keyboard", "Escape pressed - clearing search");
                return Task::done(Message::SearchQueryChanged(String::new()));
            }
            // Future: close other panels
        }

        // Ctrl+F: Focus search (we'll just clear and let user type)
        keyboard::Key::Character(c) => {
            if modifiers.control() && c == "f" {
                tracing::debug!(target: "ui::keyboard", "Ctrl+F pressed - focus search");
                // For now, clear search to indicate focus
                // Proper focus management needs widget ID tracking
                return Task::done(Message::SearchQueryChanged(String::new()));
            }
        }

        _ => {}
    }

    Task::none()
}
