//! Search bar, filter chips, and track count/sort controls.

use iced::widget::{Space, button, container, row, text, text_input};
use iced::{Element, Length};

use crate::ui::icons::{self, icon_sized};
use crate::ui::messages::Message;
use crate::ui::state::{LoadedState, SortColumn};
use crate::ui::theme::{self, color, radius, spacing, typography};

/// Renders the search bar with icon and filter chips
pub fn search_and_filters(state: &LoadedState) -> Element<'_, Message> {
    // Search input with icon
    let search_row = container(
        row![
            container(icon_sized(icons::SEARCH, typography::SIZE_BODY).color(color::TEXT_MUTED))
                .padding([0, spacing::SM]),
            text_input("Search tracks, artists, albums...", &state.search_query)
                .on_input(Message::SearchQueryChanged)
                .padding(spacing::SM)
                .width(Length::Fill)
                .style(search_input_style),
        ]
        .align_y(iced::Alignment::Center),
    )
    .style(|_| container::Style {
        background: Some(iced::Background::Color(color::SURFACE)),
        border: iced::Border {
            color: color::BORDER_SUBTLE,
            width: 1.0,
            radius: radius::SM.into(),
        },
        ..Default::default()
    })
    .padding([0, spacing::SM])
    .width(Length::FillPortion(3));

    // Filter chips
    let format_filters = ["FLAC", "MP3", "WAV", "OGG"];
    let format_chips: Vec<Element<Message>> = format_filters
        .iter()
        .map(|&fmt| {
            let is_active = state.filter_format.as_deref() == Some(fmt);
            let msg = if is_active {
                Message::FilterByFormat(None)
            } else {
                Message::FilterByFormat(Some(fmt.to_string()))
            };
            filter_chip(fmt, is_active, msg)
        })
        .collect();

    // Lossless filter chip
    let lossless_active = state.filter_lossless == Some(true);
    let lossless_chip = filter_chip(
        "Lossless",
        lossless_active,
        if lossless_active {
            Message::FilterByLossless(None)
        } else {
            Message::FilterByLossless(Some(true))
        },
    );

    // Clear filters button (only show when filters active)
    let has_filters = !state.search_query.is_empty()
        || state.filter_format.is_some()
        || state.filter_lossless.is_some();

    let clear_btn: Element<Message> = if has_filters {
        button(
            row![
                icon_sized(icons::XMARK, typography::SIZE_TINY).color(color::TEXT_MUTED),
                Space::with_width(spacing::XS),
                text("Clear")
                    .size(typography::SIZE_TINY)
                    .color(color::TEXT_MUTED),
            ]
            .align_y(iced::Alignment::Center),
        )
        .padding([spacing::XS, spacing::SM])
        .style(theme::button_ghost)
        .on_press(Message::ClearFilters)
        .into()
    } else {
        Space::with_width(Length::Shrink).into()
    };

    row![
        search_row,
        Space::with_width(spacing::MD),
        row(format_chips).spacing(spacing::XS),
        Space::with_width(spacing::XS),
        lossless_chip,
        Space::with_width(Length::Fill),
        clear_btn,
    ]
    .align_y(iced::Alignment::Center)
    .into()
}

/// Creates a pill-shaped filter chip
fn filter_chip<'a>(
    label: &'static str,
    is_active: bool,
    on_press: Message,
) -> Element<'a, Message> {
    let (bg, text_color, border_color) = if is_active {
        (color::PRIMARY, color::TEXT_PRIMARY, color::PRIMARY)
    } else {
        (
            color::SURFACE_ELEVATED,
            color::TEXT_SECONDARY,
            color::BORDER_SUBTLE,
        )
    };

    button(text(label).size(typography::SIZE_TINY))
        .padding([spacing::XS, spacing::SM])
        .style(move |_theme, status| {
            let bg = match status {
                button::Status::Hovered => {
                    if is_active {
                        color::PRIMARY_HOVER
                    } else {
                        color::SURFACE_HOVER
                    }
                }
                button::Status::Pressed => {
                    if is_active {
                        color::PRIMARY_PRESSED
                    } else {
                        color::SURFACE_ELEVATED
                    }
                }
                _ => bg,
            };
            button::Style {
                background: Some(iced::Background::Color(bg)),
                text_color,
                border: iced::Border {
                    color: border_color,
                    width: 1.0,
                    radius: radius::PILL.into(),
                },
                ..Default::default()
            }
        })
        .on_press(on_press)
        .into()
}

/// Search input style (no border, transparent bg)
fn search_input_style(_theme: &iced::Theme, _status: text_input::Status) -> text_input::Style {
    text_input::Style {
        background: iced::Background::Color(iced::Color::TRANSPARENT),
        border: iced::Border::default(),
        icon: color::TEXT_MUTED,
        placeholder: color::TEXT_MUTED,
        value: color::TEXT_PRIMARY,
        selection: color::PRIMARY,
    }
}

/// Track count display and sort controls
pub fn track_count_and_sort(
    state: &LoadedState,
    filtered: usize,
    total: usize,
) -> Element<'_, Message> {
    // Track count text
    let count_text = if state.is_scanning {
        text("Scanning...")
            .size(typography::SIZE_SMALL)
            .color(color::TEXT_MUTED)
    } else if filtered == total {
        text(format!("{} tracks", format_number(total)))
            .size(typography::SIZE_SMALL)
            .color(color::TEXT_SECONDARY)
    } else {
        text(format!(
            "{} tracks (showing {})",
            format_number(total),
            format_number(filtered)
        ))
        .size(typography::SIZE_SMALL)
        .color(color::TEXT_SECONDARY)
    };

    // Sort dropdown button
    let sort_label = match state.sort_column {
        SortColumn::Title => "Title",
        SortColumn::Artist => "Artist",
        SortColumn::Album => "Album",
        SortColumn::Year => "Year",
        SortColumn::Duration => "Duration",
        SortColumn::Format => "Format",
    };
    let sort_arrow = if state.sort_ascending {
        icons::ARROW_UP
    } else {
        icons::ARROW_DOWN
    };

    // Sort options as buttons (simplified dropdown)
    let sort_btn = button(
        row![
            text(format!("Sort: {}", sort_label))
                .size(typography::SIZE_SMALL)
                .color(color::TEXT_SECONDARY),
            Space::with_width(spacing::XS),
            icon_sized(sort_arrow, typography::SIZE_TINY).color(color::TEXT_MUTED),
        ]
        .align_y(iced::Alignment::Center),
    )
    .padding([spacing::XS, spacing::SM])
    .style(theme::button_ghost)
    .on_press(Message::SortByColumn(state.sort_column)); // Clicking toggles direction

    row![count_text, Space::with_width(Length::Fill), sort_btn,]
        .align_y(iced::Alignment::Center)
        .into()
}

/// Format number with commas (e.g., 3428 -> "3,428")
pub fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.insert(0, ',');
        }
        result.insert(0, c);
    }
    result
}
