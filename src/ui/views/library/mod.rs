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

use iced::widget::{Space, button, column, container, row, text, text_input};
use iced::{Element, Length};

use crate::ui::icons::spinner_frame;
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
        // Scan progress indicator (only shown when scanning)
        scan_progress(s),
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

/// Renders the scan progress indicator (only visible during scanning)
fn scan_progress(state: &LoadedState) -> Element<'_, Message> {
    use super::loading::LoadingContext;

    if !state.is_scanning {
        return Space::with_height(0).into();
    }

    // Animated spinner character
    let spinner_char = spinner_frame(state.animation_tick);

    // Fun message that rotates
    let fun_message = LoadingContext::Scanning.message_for_tick(state.animation_tick);

    // Progress info
    let progress_text = if state.scan_count == 0 {
        fun_message.to_string()
    } else {
        format!("{} • {} files found", fun_message, state.scan_count)
    };

    container(
        row![
            // Spinner in fixed-width container to prevent jitter
            container(
                text(spinner_char)
                    .size(typography::SIZE_BODY)
                    .color(color::PRIMARY)
            )
            .width(Length::Fixed(20.0))
            .center_x(Length::Fixed(20.0)),
            Space::with_width(spacing::SM),
            // Progress count with fun message
            text(progress_text)
                .size(typography::SIZE_BODY)
                .color(color::TEXT_PRIMARY),
        ]
        .align_y(iced::Alignment::Center),
    )
    .padding([spacing::SM, spacing::MD])
    .width(Length::Fill)
    .style(|_| container::Style {
        background: Some(iced::Background::Color(color::SURFACE_ELEVATED)),
        border: iced::Border {
            color: color::BORDER_SUBTLE,
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    })
    .into()
}
