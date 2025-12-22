//! Search and filter handlers.
//!
//! Handles search query changes, column sorting, and format filtering.

use iced::Task;

use super::super::messages::Message;
use super::super::state::{LoadedState, SortColumn};
use crate::ui::views::helpers::{format_from_path, is_lossless};

/// Handle search and filter messages
pub fn handle_search_filter(s: &mut LoadedState, message: Message) -> Task<Message> {
    match message {
        Message::SearchQueryChanged(query) => {
            s.search_query = query;
            apply_filters_and_sort(s);
        }
        Message::SortByColumn(col) => {
            if s.sort_column == col {
                // Toggle sort direction if clicking same column
                s.sort_ascending = !s.sort_ascending;
            } else {
                s.sort_column = col;
                s.sort_ascending = true;
            }
            apply_filters_and_sort(s);
        }
        Message::FilterByFormat(format) => {
            s.filter_format = format;
            apply_filters_and_sort(s);
        }
        Message::FilterByLossless(lossless) => {
            s.filter_lossless = lossless;
            apply_filters_and_sort(s);
        }
        Message::ClearFilters => {
            s.search_query.clear();
            s.filter_format = None;
            s.filter_lossless = None;
            s.filtered_indices.clear();
            // Keep sort settings but rebuild indices
            apply_filters_and_sort(s);
        }
        _ => {}
    }
    Task::none()
}

/// Apply all active filters and sorting to create filtered_indices
fn apply_filters_and_sort(s: &mut LoadedState) {
    let query = s.search_query.to_lowercase();
    let has_search = !query.is_empty();
    let has_format = s.filter_format.is_some();
    let has_lossless = s.filter_lossless.is_some();

    // If no filters and default sort, clear filtered_indices
    // (track_list will iterate all tracks directly)
    if !has_search
        && !has_format
        && !has_lossless
        && s.sort_column == SortColumn::Title
        && s.sort_ascending
    {
        s.filtered_indices.clear();
        return;
    }

    // Build filtered indices
    let mut indices: Vec<usize> = s
        .tracks
        .iter()
        .enumerate()
        .filter(|(_, track)| {
            // Search filter
            if has_search {
                let title_match = track.title.to_lowercase().contains(&query);
                let artist_match = track.artist_name.to_lowercase().contains(&query);
                let album_match = track.album_name.to_lowercase().contains(&query);
                if !title_match && !artist_match && !album_match {
                    return false;
                }
            }

            // Format filter
            if let Some(ref fmt) = s.filter_format {
                let track_format = format_from_path(&track.path);
                if track_format != fmt {
                    return false;
                }
            }

            // Lossless filter
            if let Some(true) = s.filter_lossless {
                let track_format = format_from_path(&track.path);
                if !is_lossless(track_format) {
                    return false;
                }
            }

            true
        })
        .map(|(i, _)| i)
        .collect();

    // Sort indices based on current sort column
    let tracks = &s.tracks;
    let ascending = s.sort_ascending;

    indices.sort_by(|&a, &b| {
        let track_a = &tracks[a];
        let track_b = &tracks[b];

        let cmp = match s.sort_column {
            SortColumn::Title => track_a
                .title
                .to_lowercase()
                .cmp(&track_b.title.to_lowercase()),
            SortColumn::Artist => track_a
                .artist_name
                .to_lowercase()
                .cmp(&track_b.artist_name.to_lowercase()),
            SortColumn::Album => track_a
                .album_name
                .to_lowercase()
                .cmp(&track_b.album_name.to_lowercase()),
            SortColumn::Year => track_a.year.cmp(&track_b.year),
            SortColumn::Duration => track_a.duration.cmp(&track_b.duration),
            SortColumn::Format => {
                let fmt_a = format_from_path(&track_a.path);
                let fmt_b = format_from_path(&track_b.path);
                fmt_a.cmp(fmt_b)
            }
        };

        if ascending { cmp } else { cmp.reverse() }
    });

    s.filtered_indices = indices;

    // Reset scroll position when filters change
    s.scroll_offset = 0.0;
}
