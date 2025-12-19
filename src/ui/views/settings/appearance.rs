//! Appearance settings section - theme settings (placeholder for future).

use iced::widget::{Space, column, container, row, text};
use iced::{Alignment, Element, Length};

use crate::ui::icons;
use crate::ui::messages::Message;
use crate::ui::state::LoadedState;
use crate::ui::theme::{color, radius, spacing, typography};

use super::{section_header, setting_description, setting_label};

/// Appearance settings section
pub fn appearance_section(_s: &LoadedState) -> Element<'_, Message> {
    column![
        section_header(icons::PALETTE, "Appearance"),
        Space::with_height(spacing::SM),
        // Theme selector (placeholder - only dark theme for now)
        setting_row("Theme", "Color scheme for the application", theme_display(),),
        Space::with_height(spacing::MD),
        // Coming soon note
        coming_soon_note(),
    ]
    .spacing(spacing::XS)
    .into()
}

/// A setting row with label, description, and control
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

/// Current theme display (read-only for now)
fn theme_display() -> Element<'static, Message> {
    container(
        row![
            text(icons::MOON)
                .size(typography::SIZE_SMALL)
                .color(color::PRIMARY),
            Space::with_width(spacing::XS),
            text("Dark")
                .size(typography::SIZE_BODY)
                .color(color::TEXT_PRIMARY),
        ]
        .align_y(Alignment::Center),
    )
    .padding([spacing::SM, spacing::MD])
    .style(|_| container::Style {
        background: Some(color::SURFACE_ELEVATED.into()),
        border: iced::Border {
            color: color::BORDER,
            width: 1.0,
            radius: radius::SM.into(),
        },
        ..Default::default()
    })
    .into()
}

/// Coming soon placeholder
fn coming_soon_note() -> Element<'static, Message> {
    container(
        row![
            text(icons::SPARKLE)
                .size(typography::SIZE_SMALL)
                .color(color::TEXT_MUTED),
            Space::with_width(spacing::SM),
            text("More themes coming soon!")
                .size(typography::SIZE_SMALL)
                .color(color::TEXT_MUTED),
        ]
        .align_y(Alignment::Center),
    )
    .padding(spacing::SM)
    .style(|_| container::Style {
        background: Some(color::SURFACE.into()),
        border: iced::Border {
            color: color::BORDER,
            width: 1.0,
            radius: radius::SM.into(),
        },
        ..Default::default()
    })
    .into()
}
