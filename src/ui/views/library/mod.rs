//! Library pane and related components.
//!
//! This module is split into submodules for maintainability:
//! - `search`: Search bar, filter chips, track count/sort controls
//! - `track_list`: Track table header, rows, virtualized list
//! - `organize`: File organization section (collapsible)
//! - `enrichment`: Track identification via AcoustID

mod enrichment;
mod organize;
mod search;
mod track_list;

use iced::widget::{Space, button, column, text, text_input};
use iced::{Element, Length};

use crate::ui::messages::Message;
use crate::ui::state::LoadedState;
use crate::ui::theme::{self, color, spacing, typography};

/// Library pane with scanning, organizing, and track list
pub fn library_pane(s: &LoadedState) -> Element<'_, Message> {
    let scan_path = s.scan_path.display().to_string();

    // Calculate filtered vs total counts
    let (filtered_count, total_count) =
        if s.filtered_indices.is_empty() && s.search_query.is_empty() {
            (s.tracks.len(), s.tracks.len())
        } else {
            (s.filtered_indices.len(), s.tracks.len())
        };

    column![
        // Header row with title
        text("Library")
            .size(typography::SIZE_TITLE)
            .color(color::TEXT_PRIMARY),
        Space::with_height(spacing::MD),
        // Scan controls
        scan_controls(s, scan_path),
        Space::with_height(spacing::MD),
        // Search and filters section
        search::search_and_filters(s),
        Space::with_height(spacing::SM),
        // Track count and sort controls
        search::track_count_and_sort(s, filtered_count, total_count),
        Space::with_height(spacing::SM),
        // Track table header
        track_list::track_table_header(s),
        // Track list
        track_list::track_list(s),
        Space::with_height(spacing::MD),
        // Collapsible Organize section
        organize::organize_section_collapsible(s),
    ]
    .spacing(0)
    .into()
}

/// Renders the scan controls row
fn scan_controls(state: &LoadedState, path_display: String) -> Element<'_, Message> {
    use iced::widget::row;

    let (label, msg) = if state.is_scanning {
        ("Stop", Message::ScanStopped)
    } else {
        ("Scan", Message::ScanPressed)
    };

    row![
        text_input("Path to scan", &path_display)
            .on_input(Message::PathChanged)
            .padding(spacing::SM)
            .width(Length::Fill)
            .style(theme::text_input_style),
        Space::with_width(spacing::SM),
        button(text("Browse").size(typography::SIZE_SMALL))
            .on_press(Message::PickPath)
            .padding([spacing::SM, spacing::MD])
            .style(theme::button_secondary),
        Space::with_width(spacing::XS),
        button(text(label).size(typography::SIZE_SMALL))
            .on_press(msg)
            .padding([spacing::SM, spacing::MD])
            .style(theme::button_primary),
    ]
    .align_y(iced::Alignment::Center)
    .into()
}
