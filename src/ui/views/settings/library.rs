//! Library settings section - watch paths, scan settings.

use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length};

use crate::ui::icons;
use crate::ui::messages::Message;
use crate::ui::state::LoadedState;
use crate::ui::theme::{color, radius, spacing, typography};

use super::{section_header, setting_description, setting_label};

/// Library settings section
pub fn library_section(s: &LoadedState) -> Element<'_, Message> {
    column![
        section_header(icons::MUSIC_NOTE, "Library"),
        Space::with_height(spacing::SM),
        // Watch paths display
        setting_row_vertical(
            "Watch Directories",
            "Folders being monitored for music files",
            watch_paths_list(s),
        ),
        Space::with_height(spacing::MD),
        // Watcher status
        setting_row(
            "File Watcher",
            "Automatically detect new and changed files",
            watcher_status(s),
        ),
        Space::with_height(spacing::MD),
        // Manual rescan button
        setting_row(
            "Rescan Library",
            "Force a full rescan of all watched directories",
            rescan_button(),
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

/// A setting row with control below (vertical layout for lists)
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

/// Display list of watch paths
fn watch_paths_list(s: &LoadedState) -> Element<'_, Message> {
    if s.watcher_state.watch_paths.is_empty() {
        return container(
            text("No directories configured")
                .size(typography::SIZE_SMALL)
                .color(color::TEXT_MUTED),
        )
        .padding(spacing::SM)
        .style(|_| container::Style {
            background: Some(color::SURFACE_ELEVATED.into()),
            border: iced::Border {
                color: color::BORDER,
                width: 1.0,
                radius: radius::SM.into(),
            },
            ..Default::default()
        })
        .into();
    }

    let paths: Vec<Element<'_, Message>> = s
        .watcher_state
        .watch_paths
        .iter()
        .map(|path| {
            container(
                row![
                    text(icons::FOLDER_STR)
                        .size(typography::SIZE_SMALL)
                        .color(color::TEXT_MUTED),
                    Space::with_width(spacing::SM),
                    text(path.display().to_string())
                        .size(typography::SIZE_SMALL)
                        .color(color::TEXT_PRIMARY),
                ]
                .align_y(Alignment::Center),
            )
            .padding([spacing::XS, spacing::SM])
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
        })
        .collect();

    column(paths).spacing(spacing::XS).into()
}

/// Watcher status indicator
fn watcher_status(s: &LoadedState) -> Element<'_, Message> {
    let (icon, label, color_val) = if s.watcher_state.active {
        (icons::CHECK_CIRCLE, "Active", color::SUCCESS)
    } else {
        (icons::CIRCLE_STR, "Inactive", color::TEXT_MUTED)
    };

    row![
        text(icon).size(typography::SIZE_BODY).color(color_val),
        Space::with_width(spacing::XS),
        text(label).size(typography::SIZE_BODY).color(color_val),
    ]
    .align_y(Alignment::Center)
    .into()
}

/// Rescan library button
fn rescan_button() -> Element<'static, Message> {
    button(
        row![
            text(icons::REFRESH).size(typography::SIZE_SMALL),
            Space::with_width(spacing::XS),
            text("Rescan").size(typography::SIZE_BODY),
        ]
        .align_y(Alignment::Center),
    )
    .padding([spacing::SM, spacing::MD])
    .style(secondary_button_style)
    .on_press(Message::RescanLibrary)
    .into()
}

/// Secondary button style
fn secondary_button_style(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Active => color::SURFACE_ELEVATED,
        button::Status::Hovered => color::SURFACE_HOVER,
        button::Status::Pressed => color::SURFACE,
        button::Status::Disabled => color::SURFACE,
    };

    button::Style {
        background: Some(background.into()),
        text_color: color::TEXT_PRIMARY,
        border: iced::Border {
            color: color::BORDER,
            width: 1.0,
            radius: radius::SM.into(),
        },
        ..Default::default()
    }
}
