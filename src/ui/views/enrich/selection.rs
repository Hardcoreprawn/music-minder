//! Track selection section for the enrich pane.

use iced::widget::{Space, button, checkbox, column, container, row, scrollable, text};
use iced::{Element, Length};

use crate::ui::icons::{self, icon_sized};
use crate::ui::messages::Message;
use crate::ui::state::LoadedState;
use crate::ui::theme::{self, color, spacing, typography};

/// Track selection section with checkboxes
pub fn selection_section(s: &LoadedState) -> Element<'_, Message> {
    let enrich = &s.enrichment_pane;

    // Header with add button
    let header = row![
        text("TRACKS TO PROCESS")
            .size(typography::SIZE_TINY)
            .color(color::TEXT_MUTED),
        Space::with_width(Length::Fill),
        button(
            row![
                icon_sized(icons::PLUS, typography::SIZE_TINY).color(color::TEXT_SECONDARY),
                text("Add from Library")
                    .size(typography::SIZE_TINY)
                    .color(color::TEXT_SECONDARY),
            ]
            .spacing(spacing::XS)
            .align_y(iced::Alignment::Center),
        )
        .padding([spacing::XS, spacing::SM])
        .style(theme::button_ghost)
        .on_press(Message::EnrichAddFromLibrary),
    ]
    .align_y(iced::Alignment::Center);

    // Track list
    let track_list: Element<Message> = if enrich.selected_tracks.is_empty() {
        container(
            column![
                icon_sized(icons::FOLDER_OPEN, typography::SIZE_TITLE).color(color::TEXT_MUTED),
                Space::with_height(spacing::SM),
                text("No tracks selected")
                    .size(typography::SIZE_BODY)
                    .color(color::TEXT_MUTED),
                Space::with_height(spacing::XS),
                text("Add tracks from the Library to identify them")
                    .size(typography::SIZE_SMALL)
                    .color(color::TEXT_MUTED),
            ]
            .align_x(iced::Alignment::Center)
            .spacing(0),
        )
        .padding(spacing::XL)
        .center_x(Length::Fill)
        .into()
    } else {
        let items: Vec<Element<Message>> = enrich
            .selected_tracks
            .iter()
            .enumerate()
            .map(|(i, track_idx)| {
                // Get track info from library
                let (title, artist) = if let Some(track) = s.tracks.get(*track_idx) {
                    (track.title.as_str(), track.artist_name.as_str())
                } else {
                    ("Unknown", "Unknown")
                };

                let display_text = if artist.is_empty() || artist == "Unknown" {
                    title.to_string()
                } else {
                    format!("{} - {}", title, artist)
                };

                // Check if this track is selected for processing
                let is_checked = enrich.checked_tracks.contains(&i);

                let track_checkbox = checkbox("", is_checked)
                    .on_toggle(move |checked| Message::EnrichTrackChecked(i, checked));

                let remove_btn = button(
                    icon_sized(icons::XMARK, typography::SIZE_TINY).color(color::TEXT_MUTED),
                )
                .padding([spacing::XS, spacing::SM])
                .style(theme::button_ghost)
                .on_press(Message::EnrichRemoveTrack(i));

                container(
                    row![
                        track_checkbox,
                        Space::with_width(spacing::SM),
                        text(display_text)
                            .size(typography::SIZE_SMALL)
                            .color(color::TEXT_SECONDARY),
                        Space::with_width(Length::Fill),
                        remove_btn,
                    ]
                    .align_y(iced::Alignment::Center),
                )
                .padding([spacing::XS, spacing::SM])
                .style(move |_| container::Style {
                    background: Some(iced::Background::Color(if i % 2 == 0 {
                        color::SURFACE
                    } else {
                        color::BASE
                    })),
                    ..Default::default()
                })
                .into()
            })
            .collect();

        scrollable(column(items).spacing(1))
            .height(Length::Fixed(200.0))
            .into()
    };

    // Footer with count and clear button
    let selected_count = enrich.checked_tracks.len();
    let total_count = enrich.selected_tracks.len();

    let footer = row![
        text(format!(
            "{} tracks • {} selected",
            total_count, selected_count
        ))
        .size(typography::SIZE_TINY)
        .color(color::TEXT_MUTED),
        Space::with_width(Length::Fill),
        button(
            text("Clear All")
                .size(typography::SIZE_TINY)
                .color(color::TEXT_MUTED)
        )
        .padding([spacing::XS, spacing::SM])
        .style(theme::button_ghost)
        .on_press(Message::EnrichClearTracks),
    ]
    .align_y(iced::Alignment::Center);

    container(
        column![
            header,
            Space::with_height(spacing::SM),
            track_list,
            Space::with_height(spacing::SM),
            footer,
        ]
        .spacing(0),
    )
    .padding(spacing::MD)
    .style(|_| container::Style {
        background: Some(iced::Background::Color(color::SURFACE)),
        border: iced::Border {
            color: color::BORDER_SUBTLE,
            width: 1.0,
            radius: 6.0.into(),
        },
        ..Default::default()
    })
    .width(Length::Fill)
    .into()
}
