//! Library pane and related components (track list, organize, enrichment).

use std::path::Path;

use iced::widget::{Space, button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length};

use crate::db::TrackWithMetadata;
use crate::ui::icons::{self, icon_sized};
use crate::ui::messages::Message;
use crate::ui::state::{LoadedState, OrganizeView, SortColumn, virtualization as virt};

use super::helpers::{action_button, calc_visible_range};

/// Muted text color for secondary metadata columns
const MUTED_COLOR: [f32; 3] = [0.5, 0.5, 0.55];

/// Library pane with scanning, organizing, and track list
pub fn library_pane(s: &LoadedState) -> Element<'_, Message> {
    let scan_path = s.scan_path.display().to_string();

    // Loading indicator for tracks
    let (filtered_count, total_count) =
        if s.filtered_indices.is_empty() && s.search_query.is_empty() {
            (s.tracks.len(), s.tracks.len())
        } else {
            (s.filtered_indices.len(), s.tracks.len())
        };

    let track_count_text = if s.is_scanning {
        text("Loading tracks...").size(16).color([0.6, 0.6, 0.6])
    } else if filtered_count == total_count {
        text(format!("{} tracks", total_count)).size(16)
    } else {
        text(format!("{} of {} tracks", filtered_count, total_count)).size(16)
    };

    column![
        text("Library").size(28),
        scan_controls(s, scan_path),
        search_bar(s),
        track_count_text,
        Space::with_height(10),
        track_table_header(s),
        track_list(s),
    ]
    .spacing(10)
    .into()
}

/// Renders the scan controls row
fn scan_controls(state: &LoadedState, path_display: String) -> Element<'_, Message> {
    let (label, msg) = if state.is_scanning {
        ("Stop Scan", Message::ScanStopped)
    } else {
        ("Scan Library", Message::ScanPressed)
    };
    row![
        text_input("Path to scan", &path_display)
            .on_input(Message::PathChanged)
            .padding(10)
            .width(Length::FillPortion(3)),
        button("Browse").on_press(Message::PickPath).padding(10),
        button(label).on_press(msg).padding(10),
    ]
    .spacing(10)
    .into()
}

/// Renders the search bar and filter chips
fn search_bar(state: &LoadedState) -> Element<'_, Message> {
    let search_input = text_input("Search tracks...", &state.search_query)
        .on_input(Message::SearchQueryChanged)
        .padding(8)
        .width(Length::FillPortion(3));

    // Filter chips
    let format_filters = ["FLAC", "MP3", "WAV", "OGG", "AAC"];
    let format_chips: Vec<Element<Message>> = format_filters
        .iter()
        .map(|&fmt| {
            let is_active = state.filter_format.as_deref() == Some(fmt);
            let style = if is_active {
                chip_style_active
            } else {
                chip_style_inactive
            };
            let msg = if is_active {
                Message::FilterByFormat(None)
            } else {
                Message::FilterByFormat(Some(fmt.to_string()))
            };
            button(text(fmt).size(11))
                .padding([4, 8])
                .style(style)
                .on_press(msg)
                .into()
        })
        .collect();

    // Lossless filter chip
    let lossless_active = state.filter_lossless == Some(true);
    let lossless_chip: Element<Message> = button(text("Lossless").size(11))
        .padding([4, 8])
        .style(if lossless_active {
            chip_style_active
        } else {
            chip_style_inactive
        })
        .on_press(if lossless_active {
            Message::FilterByLossless(None)
        } else {
            Message::FilterByLossless(Some(true))
        })
        .into();

    // Clear filters button (only show when filters active)
    let has_filters = !state.search_query.is_empty()
        || state.filter_format.is_some()
        || state.filter_lossless.is_some();

    let clear_btn: Element<Message> = if has_filters {
        button(text("Clear").size(11))
            .padding([4, 8])
            .on_press(Message::ClearFilters)
            .into()
    } else {
        Space::with_width(Length::Shrink).into()
    };

    row![
        search_input,
        row(format_chips).spacing(4),
        lossless_chip,
        clear_btn,
    ]
    .spacing(8)
    .into()
}

/// Active filter chip style
fn chip_style_active(_theme: &iced::Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(iced::Background::Color([0.3, 0.5, 0.7].into())),
        text_color: iced::Color::WHITE,
        border: iced::Border {
            radius: 12.0.into(),
            ..Default::default()
        },
        shadow: iced::Shadow::default(),
    }
}

/// Inactive filter chip style
fn chip_style_inactive(_theme: &iced::Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(iced::Background::Color([0.25, 0.25, 0.3].into())),
        text_color: iced::Color::from_rgb(0.7, 0.7, 0.7),
        border: iced::Border {
            radius: 12.0.into(),
            ..Default::default()
        },
        shadow: iced::Shadow::default(),
    }
}

/// Renders the organize section based on current view
pub fn organize_section(state: &LoadedState, dest: String) -> Element<'_, Message> {
    match &state.organize_view {
        OrganizeView::Input => organize_input(state, dest),
        OrganizeView::Preview => organize_preview(state, dest),
        OrganizeView::Organizing => organize_progress(state),
    }
}

/// Renders the organize input view
fn organize_input(state: &LoadedState, dest: String) -> Element<'_, Message> {
    let undo = if state.can_undo {
        Some(Message::UndoPressed)
    } else {
        None
    };
    column![
        text("Organize Files").size(20),
        row![
            text_input("Destination folder", &dest)
                .on_input(Message::OrganizeDestinationChanged)
                .padding(10)
                .width(Length::FillPortion(3)),
            button("Browse")
                .on_press(Message::PickOrganizeDestination)
                .padding(10),
        ]
        .spacing(10),
        row![
            text_input(
                "Pattern: {Artist}/{Album}/{TrackNum} - {Title}.{ext}",
                &state.organize_pattern
            )
            .on_input(Message::OrganizePatternChanged)
            .padding(10)
            .width(Length::FillPortion(3)),
            button("Preview")
                .on_press(Message::OrganizePreviewPressed)
                .padding(10),
            action_button("Undo Last", undo),
        ]
        .spacing(10),
    ]
    .spacing(10)
    .into()
}

/// Renders the organize preview view
fn organize_preview(state: &LoadedState, dest: String) -> Element<'_, Message> {
    let n = state.organize_preview.len();
    let title = if state.preview_loading {
        format!("Loading... {} files so far", n)
    } else {
        format!("Preview: {} files will be moved", n)
    };
    let confirm = if state.preview_loading {
        None
    } else {
        Some(Message::OrganizeConfirmPressed)
    };

    let header = column![
        text(title).size(20),
        text(format!("Destination: {}", dest))
            .size(12)
            .color([0.5, 0.5, 0.5]),
        row![
            button("Cancel")
                .on_press(Message::OrganizeCancelPressed)
                .padding(10),
            Space::with_width(Length::Fill),
            action_button("Organize Files", confirm),
        ]
        .spacing(10),
    ]
    .spacing(10);

    let list: Element<Message> = if n > 0 {
        virtualized_preview_list(state)
    } else {
        text("No files to organize").size(14).into()
    };
    column![header, list]
        .spacing(10)
        .height(Length::Fill)
        .into()
}

/// Renders the organizing progress view
fn organize_progress(state: &LoadedState) -> Element<'_, Message> {
    let errors = state.organize_errors.len();
    column![
        text(format!(
            "Organizing... {} of {} files",
            state.organize_progress, state.organize_total
        ))
        .size(20),
        if errors > 0 {
            text(format!("{} errors", errors))
                .size(14)
                .color([0.8, 0.4, 0.0])
        } else {
            text("").size(14)
        },
    ]
    .spacing(10)
    .into()
}

/// Renders virtualized preview list
fn virtualized_preview_list(state: &LoadedState) -> Element<'_, Message> {
    let (start, end, top, bottom) = calc_visible_range(
        state.preview_scroll_offset,
        state.preview_viewport_height,
        state.organize_preview.len(),
        virt::PREVIEW_ROW_HEIGHT,
    );
    let dest = &state.organize_destination;
    let items: Vec<_> = state.organize_preview[start..end]
        .iter()
        .map(|p| preview_item(p, dest, virt::PREVIEW_ROW_HEIGHT))
        .collect();

    scrollable(
        column![
            Space::with_height(Length::Fixed(top)),
            column(items).width(Length::Fill),
            Space::with_height(Length::Fixed(bottom)),
        ]
        .width(Length::Fill),
    )
    .height(Length::Fill)
    .width(Length::Fill)
    .on_scroll(Message::PreviewScrollChanged)
    .into()
}

/// Renders a single preview item
fn preview_item<'a>(
    p: &'a crate::organizer::OrganizePreview,
    base: &Path,
    h: f32,
) -> Element<'a, Message> {
    let from = p
        .source
        .strip_prefix(base)
        .unwrap_or(&p.source)
        .display()
        .to_string();
    let to = p
        .destination
        .strip_prefix(base)
        .unwrap_or(&p.destination)
        .display()
        .to_string();
    let same = from == to;
    let txt = if same {
        format!("{} → (no change)", from)
    } else {
        format!("{} → {}", from, to)
    };
    container(text(txt).size(12).color(if same {
        [0.5, 0.5, 0.5]
    } else {
        [0.2, 0.2, 0.2]
    }))
    .height(Length::Fixed(h))
    .width(Length::Fill)
    .into()
}

/// Renders virtualized track list with play buttons
fn track_list(state: &LoadedState) -> Element<'_, Message> {
    // Use filtered indices if filtering is active, otherwise show all tracks
    let display_indices: &[usize] = if state.filtered_indices.is_empty()
        && state.search_query.is_empty()
        && state.filter_format.is_none()
        && state.filter_lossless.is_none()
    {
        // No filtering - create indices for all tracks (done inline)
        &[]
    } else {
        &state.filtered_indices
    };

    // Get total count for virtualization
    let total_count = if display_indices.is_empty() && state.search_query.is_empty() {
        state.tracks.len()
    } else {
        display_indices.len()
    };

    let (start, end, top, bottom) = calc_visible_range(
        state.scroll_offset,
        state.viewport_height,
        total_count,
        virt::TRACK_ROW_HEIGHT,
    );

    let selected = state.enrichment.selected_track;

    // Build track rows based on whether we're filtering or not
    let items: Vec<Element<Message>> =
        if display_indices.is_empty() && state.search_query.is_empty() {
            // No filtering - iterate directly over tracks slice
            state.tracks[start..end]
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    let idx = start + i;
                    let is_selected = selected == Some(idx);
                    track_row(t, idx, is_selected)
                })
                .collect()
        } else {
            // Filtering active - use filtered indices
            display_indices[start..end]
                .iter()
                .map(|&idx| {
                    let is_selected = selected == Some(idx);
                    if let Some(t) = state.tracks.get(idx) {
                        track_row(t, idx, is_selected)
                    } else {
                        Space::with_height(Length::Fixed(virt::TRACK_ROW_HEIGHT)).into()
                    }
                })
                .collect()
        };

    scrollable(
        column![
            Space::with_height(Length::Fixed(top)),
            column(items).width(Length::Fill),
            Space::with_height(Length::Fixed(bottom)),
        ]
        .width(Length::Fill),
    )
    .height(Length::Fill)
    .width(Length::Fill)
    .on_scroll(Message::ScrollChanged)
    .into()
}

/// Renders the table header row with sortable columns
fn track_table_header(state: &LoadedState) -> Element<'_, Message> {
    row![
        // Spacer for play/queue buttons
        Space::with_width(Length::Fixed(70.0)),
        // Title column
        sortable_header("Title", SortColumn::Title, state),
        // Artist column
        sortable_header("Artist", SortColumn::Artist, state),
        // Album column
        sortable_header("Album", SortColumn::Album, state),
        // Year column
        container(sortable_header_btn("Year", SortColumn::Year, state)).width(Length::Fixed(50.0)),
        // Duration column
        container(sortable_header_btn("Duration", SortColumn::Duration, state))
            .width(Length::Fixed(60.0)),
        // Format column
        container(sortable_header_btn("Format", SortColumn::Format, state))
            .width(Length::Fixed(50.0)),
    ]
    .spacing(8)
    .padding([4, 0])
    .into()
}

/// Creates a sortable header button that fills available space
fn sortable_header<'a>(
    label: &'static str,
    col: SortColumn,
    state: &LoadedState,
) -> Element<'a, Message> {
    let is_sorted = state.sort_column == col;
    let arrow = if is_sorted {
        if state.sort_ascending { " ▲" } else { " ▼" }
    } else {
        ""
    };
    let color = if is_sorted {
        [0.9, 0.9, 0.95]
    } else {
        MUTED_COLOR
    };

    let portion = match col {
        SortColumn::Title => 3,
        SortColumn::Artist | SortColumn::Album => 2,
        _ => 1,
    };

    button(text(format!("{}{}", label, arrow)).size(12).color(color))
        .padding([2, 4])
        .style(header_btn_style)
        .on_press(Message::SortByColumn(col))
        .width(Length::FillPortion(portion))
        .into()
}

/// Creates a sortable header button for fixed-width columns
fn sortable_header_btn<'a>(
    label: &'static str,
    col: SortColumn,
    state: &LoadedState,
) -> Element<'a, Message> {
    let is_sorted = state.sort_column == col;
    let arrow = if is_sorted {
        if state.sort_ascending { " ▲" } else { " ▼" }
    } else {
        ""
    };
    let color = if is_sorted {
        [0.9, 0.9, 0.95]
    } else {
        MUTED_COLOR
    };

    button(text(format!("{}{}", label, arrow)).size(12).color(color))
        .padding([2, 4])
        .style(header_btn_style)
        .on_press(Message::SortByColumn(col))
        .into()
}

/// Style for header buttons (transparent background)
fn header_btn_style(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Some(iced::Background::Color([0.25, 0.25, 0.3].into())),
        _ => None,
    };
    button::Style {
        background: bg,
        text_color: iced::Color::WHITE,
        border: iced::Border::default(),
        shadow: iced::Shadow::default(),
    }
}

/// Format duration as mm:ss
fn format_duration(seconds: Option<i64>) -> String {
    match seconds {
        Some(s) => {
            let mins = s / 60;
            let secs = s % 60;
            format!("{}:{:02}", mins, secs)
        }
        None => "-:--".to_string(),
    }
}

/// Extract format from file extension
fn format_from_path(path: &str) -> &'static str {
    if let Some(ext) = Path::new(path).extension() {
        match ext.to_string_lossy().to_lowercase().as_str() {
            "flac" => "FLAC",
            "wav" => "WAV",
            "mp3" => "MP3",
            "m4a" | "aac" => "AAC",
            "ogg" | "oga" => "OGG",
            "opus" => "OPUS",
            "wv" => "WV",
            "ape" => "APE",
            "aiff" | "aif" => "AIFF",
            _ => "?",
        }
    } else {
        "?"
    }
}

/// Renders a single track row in table format
fn track_row(t: &TrackWithMetadata, idx: usize, is_selected: bool) -> Element<'_, Message> {
    let bg_color = if is_selected {
        [0.25, 0.35, 0.45]
    } else {
        [0.18, 0.18, 0.22]
    };
    let text_color = if is_selected {
        [0.9, 0.95, 1.0]
    } else {
        [0.85, 0.85, 0.85]
    };
    let muted = if is_selected {
        [0.7, 0.75, 0.8]
    } else {
        MUTED_COLOR
    };

    let year_str = t.year.map(|y| y.to_string()).unwrap_or_default();
    let duration_str = format_duration(t.duration);
    let format_str = format_from_path(&t.path);

    let row_content = row![
        // Play button
        button(text(">").size(12))
            .padding([4, 8])
            .on_press(Message::PlayerPlayTrack(idx)),
        // Queue button
        button(text("+").size(14))
            .padding([4, 8])
            .on_press(Message::PlayerQueueTrack(idx)),
        // Title - primary emphasis
        container(text(&t.title).size(14).color(text_color))
            .width(Length::FillPortion(3))
            .center_y(Length::Fixed(virt::TRACK_ROW_HEIGHT)),
        // Artist - primary emphasis
        container(text(&t.artist_name).size(14).color(text_color))
            .width(Length::FillPortion(2))
            .center_y(Length::Fixed(virt::TRACK_ROW_HEIGHT)),
        // Album - muted
        container(text(&t.album_name).size(12).color(muted))
            .width(Length::FillPortion(2))
            .center_y(Length::Fixed(virt::TRACK_ROW_HEIGHT)),
        // Year - muted
        container(text(year_str).size(12).color(muted))
            .width(Length::Fixed(50.0))
            .center_y(Length::Fixed(virt::TRACK_ROW_HEIGHT)),
        // Duration - muted
        container(text(duration_str).size(12).color(muted))
            .width(Length::Fixed(60.0))
            .center_y(Length::Fixed(virt::TRACK_ROW_HEIGHT)),
        // Format - muted
        container(text(format_str).size(11).color(muted))
            .width(Length::Fixed(50.0))
            .center_y(Length::Fixed(virt::TRACK_ROW_HEIGHT)),
    ]
    .spacing(8);

    button(
        container(row_content)
            .height(Length::Fixed(virt::TRACK_ROW_HEIGHT))
            .width(Length::Fill),
    )
    .style(move |_theme, _status| button::Style {
        background: Some(iced::Background::Color(bg_color.into())),
        text_color: iced::Color::from_rgb(text_color[0], text_color[1], text_color[2]),
        border: iced::Border::default(),
        shadow: iced::Shadow::default(),
    })
    .padding(0)
    .width(Length::Fill)
    .on_press(Message::EnrichmentTrackSelected(idx))
    .into()
}

/// Renders the enrichment section
pub fn enrichment_section(state: &LoadedState) -> Element<'_, Message> {
    let e = &state.enrichment;

    // Tool status indicator
    let tool_status: Element<Message> = if e.fpcalc_available {
        row![
            icon_sized(icons::CHECK, 12).color([0.2, 0.6, 0.2]),
            text(" fpcalc ready").size(12).color([0.2, 0.6, 0.2])
        ]
        .into()
    } else {
        row![
            icon_sized(icons::X, 12).color([0.8, 0.2, 0.2]),
            text(" fpcalc missing").size(12).color([0.8, 0.2, 0.2])
        ]
        .into()
    };

    // API key input
    let api_key_input = text_input("AcoustID API Key", &e.api_key)
        .on_input(Message::EnrichmentApiKeyChanged)
        .padding(8)
        .width(Length::Fill);

    // Selected track display
    let selected_text = if let Some(idx) = e.selected_track {
        if let Some(track) = state.tracks.get(idx) {
            format!("Selected: {} - {}", track.artist_name, track.title)
        } else {
            "No track selected".to_string()
        }
    } else {
        "Click a track to select".to_string()
    };

    // Identify button
    let can_identify = e.selected_track.is_some()
        && !e.is_identifying
        && e.fpcalc_available
        && !e.api_key.is_empty();
    let identify_btn = if can_identify {
        button("Identify Track")
            .padding(8)
            .on_press(Message::EnrichmentIdentifyPressed)
    } else if e.is_identifying {
        button("Identifying...").padding(8)
    } else {
        button("Identify Track").padding(8)
    };

    // Result display
    let result_view: Element<Message> = if let Some(ref result) = e.last_result {
        let track = &result.track;
        let write_btn = button("Write Tags to File")
            .padding(8)
            .on_press(Message::EnrichmentWriteTagsPressed);
        column![
            text(format!("Match: {:.0}% confidence", result.score * 100.0))
                .size(14)
                .color([0.2, 0.6, 0.2]),
            text(format!("Title: {}", track.title.as_deref().unwrap_or("-"))).size(12),
            text(format!(
                "Artist: {}",
                track.artist.as_deref().unwrap_or("-")
            ))
            .size(12),
            text(format!("Album: {}", track.album.as_deref().unwrap_or("-"))).size(12),
            if let Some(year) = track.year {
                text(format!("Year: {}", year)).size(12)
            } else {
                text("").size(12)
            },
            Space::with_height(Length::Fixed(5.0)),
            write_btn,
        ]
        .spacing(2)
        .into()
    } else if let Some(ref err) = e.last_error {
        text(format!("Error: {}", err))
            .size(12)
            .color([0.8, 0.2, 0.2])
            .into()
    } else {
        text("").size(12).into()
    };

    column![
        text("Identify Track").size(20),
        tool_status,
        api_key_input,
        text(selected_text).size(12).color([0.5, 0.5, 0.5]),
        identify_btn,
        result_view,
    ]
    .spacing(8)
    .into()
}
