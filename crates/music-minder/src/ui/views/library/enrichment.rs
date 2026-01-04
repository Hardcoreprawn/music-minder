//! Track identification and enrichment via AcoustID.
//!
//! Note: This module is kept for potential future integration but currently
//! the enrichment UI lives in the settings pane.

#![allow(dead_code)]

use iced::widget::{Space, button, column, row, text, text_input};
use iced::{Element, Length};

use crate::ui::icons::{self, icon_sized};
use crate::ui::messages::Message;
use crate::ui::state::LoadedState;
use crate::ui::theme::{self, color, spacing, typography};

/// Renders the enrichment section
pub fn enrichment_section(state: &LoadedState) -> Element<'_, Message> {
    let e = &state.enrichment;

    // Tool status indicator
    let tool_status: Element<Message> = if e.fpcalc_available {
        row![
            icon_sized(icons::CIRCLE_CHECK, typography::SIZE_SMALL).color(color::SUCCESS),
            Space::with_width(spacing::XS),
            text("fpcalc ready")
                .size(typography::SIZE_SMALL)
                .color(color::SUCCESS)
        ]
        .align_y(iced::Alignment::Center)
        .into()
    } else {
        row![
            icon_sized(icons::CIRCLE_XMARK, typography::SIZE_SMALL).color(color::ERROR),
            Space::with_width(spacing::XS),
            text("fpcalc missing")
                .size(typography::SIZE_SMALL)
                .color(color::ERROR)
        ]
        .align_y(iced::Alignment::Center)
        .into()
    };

    // API key input
    let api_key_input = text_input("AcoustID API Key", &e.api_key)
        .on_input(Message::EnrichmentApiKeyChanged)
        .padding(spacing::SM)
        .width(Length::Fill)
        .style(theme::text_input_style);

    // Selected track display
    let selected_text = if let Some(idx) = e.selected_track {
        if let Some(track) = state.tracks.get(idx) {
            format!("Selected: {} - {}", track.artist_name, track.title)
        } else {
            "No track selected".to_string()
        }
    } else {
        "Click a track in the library to select".to_string()
    };

    // Identify button
    let can_identify = e.selected_track.is_some()
        && !e.is_identifying
        && e.fpcalc_available
        && !e.api_key.is_empty();

    let identify_btn = if e.is_identifying {
        button(text("Identifying...").size(typography::SIZE_SMALL))
            .padding([spacing::SM, spacing::MD])
            .style(theme::button_secondary)
    } else if can_identify {
        button(text("Identify Track").size(typography::SIZE_SMALL))
            .padding([spacing::SM, spacing::MD])
            .style(theme::button_primary)
            .on_press(Message::EnrichmentIdentifyPressed)
    } else {
        button(text("Identify Track").size(typography::SIZE_SMALL))
            .padding([spacing::SM, spacing::MD])
            .style(theme::button_secondary)
    };

    // Result display
    let result_view: Element<Message> = if let Some(ref result) = e.last_result {
        let track = &result.track;
        let write_btn = button(text("Write Tags to File").size(typography::SIZE_SMALL))
            .padding([spacing::SM, spacing::MD])
            .style(theme::button_primary)
            .on_press(Message::EnrichmentWriteTagsPressed);

        column![
            text(format!("Match: {:.0}% confidence", result.score * 100.0))
                .size(typography::SIZE_SMALL)
                .color(color::SUCCESS),
            text(format!("Title: {}", track.title.as_deref().unwrap_or("-")))
                .size(typography::SIZE_SMALL)
                .color(color::TEXT_SECONDARY),
            text(format!(
                "Artist: {}",
                track.artist.as_deref().unwrap_or("-")
            ))
            .size(typography::SIZE_SMALL)
            .color(color::TEXT_SECONDARY),
            text(format!("Album: {}", track.album.as_deref().unwrap_or("-")))
                .size(typography::SIZE_SMALL)
                .color(color::TEXT_SECONDARY),
            if let Some(year) = track.year {
                text(format!("Year: {}", year))
                    .size(typography::SIZE_SMALL)
                    .color(color::TEXT_SECONDARY)
            } else {
                text("").size(typography::SIZE_SMALL)
            },
            Space::with_height(spacing::SM),
            write_btn,
        ]
        .spacing(spacing::XS)
        .into()
    } else if let Some(ref err) = e.last_error {
        text(format!("Error: {}", err))
            .size(typography::SIZE_SMALL)
            .color(color::ERROR)
            .into()
    } else {
        text("").size(typography::SIZE_SMALL).into()
    };

    column![
        text("Identify Track")
            .size(typography::SIZE_HEADING)
            .color(color::TEXT_PRIMARY),
        Space::with_height(spacing::SM),
        tool_status,
        Space::with_height(spacing::SM),
        api_key_input,
        Space::with_height(spacing::SM),
        text(selected_text)
            .size(typography::SIZE_SMALL)
            .color(color::TEXT_MUTED),
        Space::with_height(spacing::SM),
        identify_btn,
        Space::with_height(spacing::SM),
        result_view,
    ]
    .spacing(0)
    .into()
}
