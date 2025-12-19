//! Enrichment settings section - AcoustID API key, fpcalc status.

use iced::widget::{Space, column, container, row, text, text_input};
use iced::{Alignment, Element, Length};

use crate::ui::icons;
use crate::ui::messages::Message;
use crate::ui::state::LoadedState;
use crate::ui::theme::{color, radius, spacing, typography};

use super::{section_header, setting_description, setting_label};

/// Enrichment settings section
pub fn enrichment_section(s: &LoadedState) -> Element<'_, Message> {
    column![
        section_header(icons::WAND_STR, "Enrichment"),
        Space::with_height(spacing::SM),
        // fpcalc status
        setting_row(
            "Chromaprint (fpcalc)",
            "Required for audio fingerprinting",
            fpcalc_status(s),
        ),
        Space::with_height(spacing::MD),
        // AcoustID API key
        setting_row_vertical(
            "AcoustID API Key",
            "Required for track identification. Get one free at acoustid.org",
            api_key_input(s),
        ),
    ]
    .spacing(spacing::XS)
    .into()
}

/// A setting row with label, description, and control (horizontal layout)
fn setting_row<'a>(
    label: &'a str,
    description: &'a str,
    control: Element<'a, Message>,
) -> Element<'a, Message> {
    row![
        column![setting_label(label), setting_description(description),]
            .spacing(2)
            .width(Length::FillPortion(2)),
        container(control)
            .width(Length::FillPortion(1))
            .align_x(iced::alignment::Horizontal::Right),
    ]
    .align_y(Alignment::Center)
    .spacing(spacing::MD)
    .padding([spacing::SM, 0])
    .into()
}

/// A setting row with control below (vertical layout)
fn setting_row_vertical<'a>(
    label: &'a str,
    description: &'a str,
    control: Element<'a, Message>,
) -> Element<'a, Message> {
    column![
        setting_label(label),
        setting_description(description),
        Space::with_height(spacing::SM),
        control,
    ]
    .spacing(2)
    .padding([spacing::SM, 0])
    .into()
}

/// fpcalc availability status
fn fpcalc_status(s: &LoadedState) -> Element<'_, Message> {
    let (icon, label, color_val) = if s.enrichment.fpcalc_available {
        (icons::CHECK_CIRCLE, "Installed", color::SUCCESS)
    } else {
        (icons::WARNING, "Not Found", color::WARNING)
    };

    row![
        text(icon).size(typography::SIZE_BODY).color(color_val),
        Space::with_width(spacing::XS),
        text(label).size(typography::SIZE_BODY).color(color_val),
    ]
    .align_y(Alignment::Center)
    .into()
}

/// API key input field
fn api_key_input(s: &LoadedState) -> Element<'_, Message> {
    let has_key = !s.enrichment.api_key.is_empty();

    row![
        text_input("Enter your AcoustID API key...", &s.enrichment.api_key)
            .on_input(Message::EnrichmentApiKeyChanged)
            .padding(spacing::SM)
            .size(typography::SIZE_BODY)
            .width(Length::Fill)
            .style(api_key_input_style),
        Space::with_width(spacing::SM),
        // Status indicator
        if has_key {
            text(icons::CHECK_CIRCLE)
                .size(typography::SIZE_BODY)
                .color(color::SUCCESS)
        } else {
            text(icons::CIRCLE_STR)
                .size(typography::SIZE_BODY)
                .color(color::TEXT_MUTED)
        },
    ]
    .align_y(Alignment::Center)
    .into()
}

/// Styled text input for API key
fn api_key_input_style(_theme: &iced::Theme, status: text_input::Status) -> text_input::Style {
    let border_color = match status {
        text_input::Status::Active => color::BORDER,
        text_input::Status::Hovered => color::TEXT_MUTED,
        text_input::Status::Focused => color::PRIMARY,
        text_input::Status::Disabled => color::SURFACE,
    };

    text_input::Style {
        background: color::SURFACE_ELEVATED.into(),
        border: iced::Border {
            color: border_color,
            width: 1.0,
            radius: radius::SM.into(),
        },
        icon: color::TEXT_MUTED,
        placeholder: color::TEXT_MUTED,
        value: color::TEXT_PRIMARY,
        selection: color::PRIMARY,
    }
}
