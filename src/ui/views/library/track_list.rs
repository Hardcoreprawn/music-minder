//! Track list table with virtualization, headers, and row rendering.

use iced::widget::{Space, button, column, container, row, scrollable, text, tooltip};
use iced::{Element, Length};

use crate::db::TrackWithMetadata;
#[allow(unused_imports)]
use crate::health::QualityFlags;
use crate::player::format_duration_secs;
use crate::ui::icons::{self, icon_sized};
use crate::ui::messages::Message;
use crate::ui::state::{LoadedState, SortColumn, virtualization as virt};
use crate::ui::theme::{self, color, radius, spacing, typography};
use crate::ui::views::helpers::{calc_visible_range, format_from_path, is_lossless};

/// Renders virtualized track list with play buttons
pub fn track_list(state: &LoadedState) -> Element<'_, Message> {
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

    // Enrichment selection (for batch operations)
    let enrichment_selected = state.enrichment.selected_track;
    // Keyboard navigation selection (visual_idx is index into display list)
    let keyboard_selection = state.library_selection;

    // Build track rows based on whether we're filtering or not
    let items: Vec<Element<Message>> =
        if display_indices.is_empty() && state.search_query.is_empty() {
            // No filtering - iterate directly over tracks slice
            state.tracks[start..end]
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    let idx = start + i; // actual track index
                    let visual_idx = idx; // visual index (same when not filtering)
                    let is_enrichment_selected = enrichment_selected == Some(idx);
                    let is_keyboard_selected = keyboard_selection == Some(visual_idx);
                    track_row(
                        t,
                        idx,
                        is_enrichment_selected,
                        is_keyboard_selected,
                        visual_idx,
                    )
                })
                .collect()
        } else {
            // Filtering active - use filtered indices
            display_indices[start..end]
                .iter()
                .enumerate()
                .map(|(i, &idx)| {
                    let visual_idx = start + i; // index in displayed list
                    let is_enrichment_selected = enrichment_selected == Some(idx);
                    let is_keyboard_selected = keyboard_selection == Some(visual_idx);
                    if let Some(t) = state.tracks.get(idx) {
                        track_row(
                            t,
                            idx,
                            is_enrichment_selected,
                            is_keyboard_selected,
                            visual_idx,
                        )
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
    .style(theme::scrollbar_style)
    .into()
}

/// Renders the table header row with sortable columns
pub fn track_table_header(state: &LoadedState) -> Element<'_, Message> {
    container(
        row![
            // Spacer for play/queue buttons
            Space::with_width(Length::Fixed(70.0)),
            // Quality column (non-sortable for now) with tooltip
            tooltip(
                container(
                    text("Q")
                        .size(typography::SIZE_TINY)
                        .color(color::TEXT_MUTED)
                )
                .width(Length::Fixed(20.0))
                .center_x(Length::Fixed(20.0)),
                column![
                    text("Quality Score").size(typography::SIZE_SMALL),
                    text("Metadata completeness (0-100%)").size(typography::SIZE_TINY),
                    Space::with_height(spacing::XS),
                    text("★ 90%+ Excellent")
                        .size(typography::SIZE_TINY)
                        .color(color::SUCCESS),
                    text("● 70%+ Good")
                        .size(typography::SIZE_TINY)
                        .color(color::TEXT_SECONDARY),
                    text("◐ 50%+ Fair - some issues")
                        .size(typography::SIZE_TINY)
                        .color(color::WARNING),
                    text("○ <50% Needs attention")
                        .size(typography::SIZE_TINY)
                        .color(color::ERROR),
                    Space::with_height(spacing::XS),
                    text("Deductions:")
                        .size(typography::SIZE_TINY)
                        .color(color::TEXT_MUTED),
                    text("-10 No MusicBrainz ID")
                        .size(typography::SIZE_TINY)
                        .color(color::TEXT_MUTED),
                    text("-10 Never fingerprinted")
                        .size(typography::SIZE_TINY)
                        .color(color::TEXT_MUTED),
                    text("-5 Missing year/track#")
                        .size(typography::SIZE_TINY)
                        .color(color::TEXT_MUTED),
                ]
                .spacing(2),
                tooltip::Position::Bottom,
            )
            .gap(spacing::XS)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(color::SURFACE_ELEVATED)),
                border: iced::Border {
                    color: color::BORDER_SUBTLE,
                    width: 1.0,
                    radius: radius::SM.into(),
                },
                ..Default::default()
            }),
            // Title column
            sortable_header("Title", SortColumn::Title, state, 3),
            // Artist column
            sortable_header("Artist", SortColumn::Artist, state, 2),
            // Album column
            sortable_header("Album", SortColumn::Album, state, 2),
            // Year column
            container(sortable_header_btn("Year", SortColumn::Year, state))
                .width(Length::Fixed(50.0)),
            // Duration column
            container(sortable_header_btn("Time", SortColumn::Duration, state))
                .width(Length::Fixed(60.0)),
            // Format column
            container(sortable_header_btn("Format", SortColumn::Format, state))
                .width(Length::Fixed(60.0)),
        ]
        .spacing(spacing::SM),
    )
    .padding([spacing::XS, 0])
    .style(|_| container::Style {
        background: Some(iced::Background::Color(color::SURFACE)),
        border: iced::Border {
            color: color::BORDER_SUBTLE,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    })
    .into()
}

/// Creates a sortable header button that fills available space
fn sortable_header<'a>(
    label: &'static str,
    col: SortColumn,
    state: &LoadedState,
    portion: u16,
) -> Element<'a, Message> {
    let is_sorted = state.sort_column == col;
    let arrow = if is_sorted {
        if state.sort_ascending {
            icons::ARROW_UP
        } else {
            icons::ARROW_DOWN
        }
    } else {
        ' '
    };
    let text_color = if is_sorted {
        color::TEXT_PRIMARY
    } else {
        color::TEXT_MUTED
    };

    button(
        row![
            text(label).size(typography::SIZE_SMALL).color(text_color),
            if is_sorted {
                container(icon_sized(arrow, typography::SIZE_TINY).color(text_color))
                    .padding([0, spacing::XS])
            } else {
                container(Space::with_width(0))
            },
        ]
        .align_y(iced::Alignment::Center),
    )
    .padding([spacing::XS, spacing::SM])
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
    let text_color = if is_sorted {
        color::TEXT_PRIMARY
    } else {
        color::TEXT_MUTED
    };

    button(text(label).size(typography::SIZE_SMALL).color(text_color))
        .padding([spacing::XS, spacing::SM])
        .style(header_btn_style)
        .on_press(Message::SortByColumn(col))
        .into()
}

/// Style for header buttons
fn header_btn_style(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Some(iced::Background::Color(color::SURFACE_HOVER)),
        _ => None,
    };
    button::Style {
        background: bg,
        text_color: color::TEXT_SECONDARY,
        border: iced::Border::default(),
        ..Default::default()
    }
}

/// Renders a single track row with hover states and format badges
///
/// - `is_enrichment_selected`: Track is selected for enrichment operations
/// - `is_keyboard_selected`: Track is selected via keyboard navigation (visual focus)
/// - `visual_idx`: Index in the displayed list (for keyboard navigation selection)
fn track_row(
    t: &TrackWithMetadata,
    idx: usize,
    is_enrichment_selected: bool,
    is_keyboard_selected: bool,
    visual_idx: usize,
) -> Element<'_, Message> {
    let format_str = format_from_path(&t.path);
    let lossless = is_lossless(format_str);

    // Format badge colors - subtle differentiation
    let (badge_bg, badge_text) = if lossless {
        (color::SURFACE_ELEVATED, color::SUCCESS) // Green text, subtle bg
    } else {
        (color::SURFACE_ELEVATED, color::TEXT_MUTED)
    };

    let year_str = t.year.map(|y| y.to_string()).unwrap_or_default();
    let duration_str = format_duration_secs(t.duration.unwrap_or(0) as f32);

    // Row background based on selection and alternating
    // Priority: keyboard selection > enrichment selection > alternating
    let base_bg = if is_keyboard_selected {
        color::PRIMARY // Bright highlight for keyboard focus
    } else if is_enrichment_selected {
        color::PRIMARY_PRESSED
    } else if visual_idx.is_multiple_of(2) {
        color::BASE
    } else {
        color::SURFACE
    };

    let text_color = if is_keyboard_selected || is_enrichment_selected {
        color::TEXT_PRIMARY
    } else {
        color::TEXT_SECONDARY
    };
    let muted_color = if is_keyboard_selected || is_enrichment_selected {
        color::TEXT_SECONDARY
    } else {
        color::TEXT_MUTED
    };

    // Left border indicator for keyboard selection
    let selection_indicator = if is_keyboard_selected {
        container(Space::with_width(3))
            .height(Length::Fixed(virt::TRACK_ROW_HEIGHT))
            .style(|_| container::Style {
                background: Some(iced::Background::Color(color::PRIMARY)),
                ..Default::default()
            })
    } else {
        container(Space::with_width(3)) // Same width placeholder to keep alignment
    };

    // Quality indicator
    let quality_indicator = quality_badge(t);

    let row_content = row![
        // Selection indicator (left edge highlight)
        selection_indicator,
        // Play button
        button(icon_sized(icons::PLAY, typography::SIZE_TINY).color(color::TEXT_MUTED))
            .padding([spacing::XS, spacing::SM])
            .style(theme::button_ghost)
            .on_press(Message::PlayerPlayTrack(idx)),
        // Queue button
        button(icon_sized(icons::PLUS, typography::SIZE_TINY).color(color::TEXT_MUTED))
            .padding([spacing::XS, spacing::SM])
            .style(theme::button_ghost)
            .on_press(Message::PlayerQueueTrack(idx)),
        // Quality indicator
        quality_indicator,
        // Title
        container(
            text(&t.title)
                .size(typography::SIZE_SMALL)
                .color(text_color)
        )
        .width(Length::FillPortion(3))
        .center_y(Length::Fixed(virt::TRACK_ROW_HEIGHT)),
        // Artist
        container(
            text(&t.artist_name)
                .size(typography::SIZE_SMALL)
                .color(text_color)
        )
        .width(Length::FillPortion(2))
        .center_y(Length::Fixed(virt::TRACK_ROW_HEIGHT)),
        // Album
        container(
            text(&t.album_name)
                .size(typography::SIZE_TINY)
                .color(muted_color)
        )
        .width(Length::FillPortion(2))
        .center_y(Length::Fixed(virt::TRACK_ROW_HEIGHT)),
        // Year
        container(
            text(year_str)
                .size(typography::SIZE_TINY)
                .color(muted_color)
        )
        .width(Length::Fixed(50.0))
        .center_y(Length::Fixed(virt::TRACK_ROW_HEIGHT)),
        // Duration
        container(
            text(duration_str)
                .size(typography::SIZE_TINY)
                .color(muted_color)
        )
        .width(Length::Fixed(60.0))
        .center_y(Length::Fixed(virt::TRACK_ROW_HEIGHT)),
        // Format badge
        container(
            container(
                text(format_str)
                    .size(typography::SIZE_TINY)
                    .color(badge_text)
            )
            .padding([2, spacing::XS])
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(badge_bg)),
                border: iced::Border {
                    radius: radius::SM.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
        )
        .width(Length::Fixed(60.0))
        .center_y(Length::Fixed(virt::TRACK_ROW_HEIGHT))
        .center_x(Length::Fixed(60.0)),
        // Context menu button (opens track detail for now, will become dropdown)
        button(icon_sized(icons::ELLIPSIS_V, typography::SIZE_SMALL).color(color::TEXT_MUTED))
            .padding([spacing::XS, spacing::SM])
            .style(theme::button_ghost)
            .on_press(Message::TrackDetailOpen(idx)),
        // Right padding to match left side and avoid scrollbar
        Space::with_width(spacing::SM),
    ]
    .spacing(spacing::SM)
    .align_y(iced::Alignment::Center);

    // Wrap in button for hover effect and selection
    // Click selects the track for keyboard navigation
    button(
        container(row_content)
            .height(Length::Fixed(virt::TRACK_ROW_HEIGHT))
            .width(Length::Fill),
    )
    .style(move |_theme, status| {
        let bg = match status {
            button::Status::Hovered => color::SURFACE_HOVER,
            button::Status::Pressed => color::SURFACE_ELEVATED,
            _ => base_bg,
        };
        button::Style {
            background: Some(iced::Background::Color(bg)),
            text_color,
            border: iced::Border::default(),
            ..Default::default()
        }
    })
    .padding(0)
    .width(Length::Fill)
    .on_press(Message::LibrarySelectIndex(visual_idx))
    .into()
}

/// Render quality indicator badge with tooltip
fn quality_badge(t: &TrackWithMetadata) -> Element<'_, Message> {
    let (icon, icon_color, tooltip_text) = match t.quality_score {
        None => {
            // Never checked
            ('?', color::TEXT_MUTED, "Not yet analyzed".to_string())
        }
        Some(score) if score >= 90 => ('★', color::SUCCESS, format!("Excellent ({}%)", score)),
        Some(score) if score >= 70 => ('●', color::TEXT_SECONDARY, format!("Good ({}%)", score)),
        Some(score) if score >= 50 => {
            // Build tooltip with specific issues
            let flags = t.quality_flags();
            let issues = flags.descriptions().join(", ");
            let tip = if issues.is_empty() {
                format!("Fair ({}%)", score)
            } else {
                format!("Fair ({}%): {}", score, issues)
            };
            ('◐', color::WARNING, tip)
        }
        Some(score) => {
            // Poor quality - show what's wrong
            let flags = t.quality_flags();
            let issues = flags.descriptions().join(", ");
            let tip = if issues.is_empty() {
                format!("Needs attention ({}%)", score)
            } else {
                format!("Needs attention ({}%): {}", score, issues)
            };
            ('○', color::ERROR, tip)
        }
    };

    tooltip(
        container(
            text(icon.to_string())
                .size(typography::SIZE_TINY)
                .color(icon_color),
        )
        .width(Length::Fixed(20.0))
        .center_x(Length::Fixed(20.0))
        .center_y(Length::Fixed(virt::TRACK_ROW_HEIGHT)),
        text(tooltip_text).size(typography::SIZE_TINY),
        tooltip::Position::Top,
    )
    .gap(spacing::XS)
    .style(|_| container::Style {
        background: Some(iced::Background::Color(color::SURFACE_ELEVATED)),
        border: iced::Border {
            color: color::BORDER_SUBTLE,
            width: 1.0,
            radius: radius::SM.into(),
        },
        ..Default::default()
    })
    .into()
}
