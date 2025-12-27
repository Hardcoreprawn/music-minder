//! Audio settings section - device selection, visualization mode.

use iced::widget::{Space, column, container, pick_list, row};
use iced::{Alignment, Element, Length};

use crate::ui::icons;
use crate::ui::messages::Message;
use crate::ui::state::{LoadedState, VisualizationMode};
use crate::ui::theme::{color, radius, spacing, typography};

use super::{section_header, setting_description, setting_label};

/// Audio settings section
pub fn audio_section(s: &LoadedState) -> Element<'_, Message> {
    column![
        section_header(icons::VOLUME_HIGH, "Audio"),
        Space::with_height(spacing::SM),
        // Output device
        setting_row(
            "Output Device",
            "Select which audio device to use for playback",
            device_picker(s),
        ),
        Space::with_height(spacing::MD),
        // Visualization mode
        setting_row(
            "Visualization",
            "Visual display mode for the Now Playing view",
            visualization_picker(s),
        ),
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

/// Audio device picker dropdown
fn device_picker(s: &LoadedState) -> Element<'_, Message> {
    let devices: Vec<String> = s.audio_devices.clone();
    let selected = if s.current_audio_device.is_empty() {
        None
    } else {
        Some(s.current_audio_device.clone())
    };

    pick_list(devices, selected, Message::PlayerSelectDevice)
        .placeholder("Default Device")
        .text_size(typography::SIZE_BODY)
        .padding(spacing::SM)
        .style(dropdown_style)
        .into()
}

/// Visualization mode picker
fn visualization_picker(s: &LoadedState) -> Element<'_, Message> {
    let modes = vec![
        VisualizationMode::Spectrum,
        VisualizationMode::Waveform,
        VisualizationMode::VuMeter,
    ];

    pick_list(
        modes,
        Some(s.visualization_mode),
        Message::PlayerVisualizationModeChanged,
    )
    .text_size(typography::SIZE_BODY)
    .padding(spacing::SM)
    .style(dropdown_style)
    .into()
}

/// Styled dropdown appearance
fn dropdown_style(_theme: &iced::Theme, status: pick_list::Status) -> pick_list::Style {
    let background = match status {
        pick_list::Status::Active => color::SURFACE_ELEVATED,
        pick_list::Status::Hovered => color::SURFACE_HOVER,
        pick_list::Status::Opened => color::SURFACE_HOVER,
    };

    pick_list::Style {
        text_color: color::TEXT_PRIMARY,
        placeholder_color: color::TEXT_MUTED,
        handle_color: color::TEXT_SECONDARY,
        background: background.into(),
        border: iced::Border {
            color: color::BORDER,
            width: 1.0,
            radius: radius::SM.into(),
        },
    }
}
