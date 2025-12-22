//! Selection and keyboard navigation handling.
//!
//! Handles track selection in library and queue views,
//! enabling keyboard navigation (Up/Down/Enter/Delete).

use iced::Task;

use super::super::messages::Message;
use super::super::state::{ActivePane, FocusedList, LoadedState};

/// Handle selection-related messages.
pub fn handle_selection(s: &mut LoadedState, message: Message) -> Task<Message> {
    match message {
        Message::LibrarySelectPrevious => {
            s.focused_list = FocusedList::Library;
            let count = visible_library_count(s);
            if count == 0 {
                return Task::none();
            }
            s.library_selection = Some(match s.library_selection {
                None => 0,    // Start at first item
                Some(0) => 0, // Stay at top
                Some(i) => i.saturating_sub(1),
            });
            tracing::debug!(target: "ui::selection", "Library selection: {:?}", s.library_selection);
        }

        Message::LibrarySelectNext => {
            s.focused_list = FocusedList::Library;
            let count = visible_library_count(s);
            if count == 0 {
                return Task::none();
            }
            let max_idx = count.saturating_sub(1);
            s.library_selection = Some(match s.library_selection {
                None => 0, // Start at first item
                Some(i) => (i + 1).min(max_idx),
            });
            tracing::debug!(target: "ui::selection", "Library selection: {:?}", s.library_selection);
        }

        Message::LibrarySelectIndex(idx) => {
            s.focused_list = FocusedList::Library;
            let count = visible_library_count(s);
            if idx < count {
                s.library_selection = Some(idx);
            }
        }

        Message::QueueSelectPrevious => {
            s.focused_list = FocusedList::Queue;
            let count = queue_count(s);
            if count == 0 {
                return Task::none();
            }
            s.queue_selection = Some(match s.queue_selection {
                None => 0,
                Some(0) => 0,
                Some(i) => i.saturating_sub(1),
            });
            tracing::debug!(target: "ui::selection", "Queue selection: {:?}", s.queue_selection);
        }

        Message::QueueSelectNext => {
            s.focused_list = FocusedList::Queue;
            let count = queue_count(s);
            if count == 0 {
                return Task::none();
            }
            let max_idx = count.saturating_sub(1);
            s.queue_selection = Some(match s.queue_selection {
                None => 0,
                Some(i) => (i + 1).min(max_idx),
            });
            tracing::debug!(target: "ui::selection", "Queue selection: {:?}", s.queue_selection);
        }

        Message::QueueSelectIndex(idx) => {
            s.focused_list = FocusedList::Queue;
            let count = queue_count(s);
            if idx < count {
                s.queue_selection = Some(idx);
            }
        }

        Message::PlaySelected => {
            // Play the selected track based on current focus and pane
            match (s.active_pane, s.focused_list) {
                (ActivePane::Library, FocusedList::Library) => {
                    if let Some(sel_idx) = s.library_selection {
                        // Convert selection index to actual track index
                        let track_idx = library_selection_to_track_index(s, sel_idx);
                        if let Some(idx) = track_idx {
                            tracing::info!(target: "ui::selection", "Playing library track at index {}", idx);
                            return Task::done(Message::PlayerPlayTrack(idx));
                        }
                    }
                }
                (ActivePane::NowPlaying, FocusedList::Queue) => {
                    if let Some(idx) = s.queue_selection {
                        tracing::info!(target: "ui::selection", "Jumping to queue track at index {}", idx);
                        return Task::done(Message::QueueJumpTo(idx));
                    }
                }
                // For other combinations, try library first if we have a selection
                _ => {
                    if let Some(sel_idx) = s.library_selection {
                        let track_idx = library_selection_to_track_index(s, sel_idx);
                        if let Some(idx) = track_idx {
                            return Task::done(Message::PlayerPlayTrack(idx));
                        }
                    }
                }
            }
        }

        Message::RemoveSelectedFromQueue => {
            // Only works when queue is focused and we're in Now Playing
            if s.active_pane == ActivePane::NowPlaying
                && s.focused_list == FocusedList::Queue
                && let Some(idx) = s.queue_selection
            {
                let count = queue_count(s);
                if idx < count {
                    tracing::info!(target: "ui::selection", "Removing queue track at index {}", idx);
                    // Adjust selection after removal
                    if count <= 1 {
                        s.queue_selection = None;
                    } else if idx >= count - 1 {
                        s.queue_selection = Some(idx.saturating_sub(1));
                    }
                    // Selection stays at same index (next item moves up)
                    return Task::done(Message::QueueRemove(idx));
                }
            }
        }

        Message::QueueMoveUp => {
            // Move selected queue item up one position
            if let Some(idx) = s.queue_selection
                && let Some(player) = &mut s.player
                && let Some(new_idx) = player.queue_mut().move_up(idx)
            {
                s.queue_selection = Some(new_idx);
                tracing::info!(target: "ui::selection", "Moved queue item {} -> {}", idx, new_idx);
            }
        }

        Message::QueueMoveDown => {
            // Move selected queue item down one position
            if let Some(idx) = s.queue_selection
                && let Some(player) = &mut s.player
                && let Some(new_idx) = player.queue_mut().move_down(idx)
            {
                s.queue_selection = Some(new_idx);
                tracing::info!(target: "ui::selection", "Moved queue item {} -> {}", idx, new_idx);
            }
        }

        _ => {}
    }

    Task::none()
}

/// Get count of visible library items (filtered or all)
fn visible_library_count(s: &LoadedState) -> usize {
    if s.filtered_indices.is_empty() && s.search_query.is_empty() {
        s.tracks.len()
    } else {
        s.filtered_indices.len()
    }
}

/// Get count of items in queue
fn queue_count(s: &LoadedState) -> usize {
    s.player.as_ref().map(|p| p.queue().len()).unwrap_or(0)
}

/// Convert a library selection index to the actual track index
/// (handles filtered vs unfiltered state)
fn library_selection_to_track_index(s: &LoadedState, sel_idx: usize) -> Option<usize> {
    if s.filtered_indices.is_empty() && s.search_query.is_empty() {
        // No filtering - selection index IS the track index
        if sel_idx < s.tracks.len() {
            Some(sel_idx)
        } else {
            None
        }
    } else {
        // Filtering active - look up in filtered_indices
        s.filtered_indices.get(sel_idx).copied()
    }
}
