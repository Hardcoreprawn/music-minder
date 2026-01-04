//! File organization section (collapsible).

use std::path::Path;

use iced::widget::{Space, button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length};

use crate::ui::icons::{self, icon_sized};
use crate::ui::messages::Message;
use crate::ui::state::{LoadedState, OrganizeView, virtualization as virt};
use crate::ui::theme::{self, color, radius, spacing, typography};
use crate::ui::views::helpers::{action_button, calc_visible_range};

/// Collapsible organize section
pub fn organize_section_collapsible(state: &LoadedState) -> Element<'_, Message> {
    let is_collapsed = state.organize_collapsed;
    let toggle_icon = if is_collapsed {
        icons::CHEVRON_RIGHT
    } else {
        icons::CHEVRON_DOWN
    };

    // Header row (always visible)
    let header = button(
        row![
            icon_sized(toggle_icon, typography::SIZE_SMALL).color(color::TEXT_MUTED),
            Space::with_width(spacing::SM),
            text("Organize Files")
                .size(typography::SIZE_HEADING)
                .color(color::TEXT_SECONDARY),
            Space::with_width(Length::Fill),
            if is_collapsed {
                text("Click to expand")
                    .size(typography::SIZE_TINY)
                    .color(color::TEXT_MUTED)
            } else {
                text("").size(typography::SIZE_TINY)
            },
        ]
        .align_y(iced::Alignment::Center),
    )
    .padding([spacing::SM, spacing::MD])
    .width(Length::Fill)
    .style(|_theme, status| {
        let bg = match status {
            button::Status::Hovered => color::SURFACE_HOVER,
            _ => color::SURFACE,
        };
        button::Style {
            background: Some(iced::Background::Color(bg)),
            text_color: color::TEXT_SECONDARY,
            border: iced::Border {
                color: color::BORDER_SUBTLE,
                width: 1.0,
                radius: radius::SM.into(),
            },
            ..Default::default()
        }
    })
    .on_press(Message::ToggleOrganizeSection);

    if is_collapsed {
        header.into()
    } else {
        let dest_path = state.organize_destination.display().to_string();
        let content = organize_section(state, dest_path);

        column![
            header,
            container(content)
                .padding([spacing::MD, spacing::LG])
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(color::SURFACE)),
                    border: iced::Border {
                        color: color::BORDER_SUBTLE,
                        width: 1.0,
                        radius: radius::SM.into(),
                    },
                    ..Default::default()
                }),
        ]
        .spacing(0)
        .into()
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
        row![
            text_input("Destination folder", &dest)
                .on_input(Message::OrganizeDestinationChanged)
                .padding(spacing::SM)
                .width(Length::Fill)
                .style(theme::text_input_style),
            Space::with_width(spacing::SM),
            button(text("Browse").size(typography::SIZE_SMALL))
                .on_press(Message::PickOrganizeDestination)
                .padding([spacing::SM, spacing::MD])
                .style(theme::button_secondary),
        ]
        .align_y(iced::Alignment::Center),
        Space::with_height(spacing::SM),
        row![
            text_input(
                "Pattern: {Artist}/{Album}/{TrackNum} - {Title}.{ext}",
                &state.organize_pattern
            )
            .on_input(Message::OrganizePatternChanged)
            .padding(spacing::SM)
            .width(Length::Fill)
            .style(theme::text_input_style),
            Space::with_width(spacing::SM),
            button(text("Preview").size(typography::SIZE_SMALL))
                .on_press(Message::OrganizePreviewPressed)
                .padding([spacing::SM, spacing::MD])
                .style(theme::button_primary),
            Space::with_width(spacing::XS),
            action_button("Undo", undo),
        ]
        .align_y(iced::Alignment::Center),
    ]
    .spacing(0)
    .into()
}

/// Renders the organize preview view
fn organize_preview(state: &LoadedState, dest: String) -> Element<'_, Message> {
    let n = state.organize_preview.len();
    let title = if state.preview_loading {
        format!("Loading... {} files so far", n)
    } else {
        format!("{} files will be moved", n)
    };
    let confirm = if state.preview_loading {
        None
    } else {
        Some(Message::OrganizeConfirmPressed)
    };

    let header = column![
        text(title)
            .size(typography::SIZE_BODY)
            .color(color::TEXT_PRIMARY),
        text(format!("Destination: {}", dest))
            .size(typography::SIZE_TINY)
            .color(color::TEXT_MUTED),
        Space::with_height(spacing::SM),
        row![
            button(text("Cancel").size(typography::SIZE_SMALL))
                .on_press(Message::OrganizeCancelPressed)
                .padding([spacing::SM, spacing::MD])
                .style(theme::button_secondary),
            Space::with_width(Length::Fill),
            action_button("Organize Files", confirm),
        ],
    ]
    .spacing(spacing::XS);

    let list: Element<Message> = if n > 0 {
        virtualized_preview_list(state)
    } else {
        text("No files to organize")
            .size(typography::SIZE_SMALL)
            .color(color::TEXT_MUTED)
            .into()
    };

    column![header, Space::with_height(spacing::SM), list]
        .spacing(0)
        .height(Length::Fixed(300.0))
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
        .size(typography::SIZE_BODY)
        .color(color::TEXT_PRIMARY),
        if errors > 0 {
            text(format!("{} errors", errors))
                .size(typography::SIZE_SMALL)
                .color(color::WARNING)
        } else {
            text("").size(typography::SIZE_SMALL)
        },
    ]
    .spacing(spacing::XS)
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
    .style(theme::scrollbar_style)
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

    let (txt, txt_color) = if same {
        (format!("{} → (no change)", from), color::TEXT_MUTED)
    } else {
        (format!("{} → {}", from, to), color::TEXT_SECONDARY)
    };

    container(text(txt).size(typography::SIZE_TINY).color(txt_color))
        .height(Length::Fixed(h))
        .width(Length::Fill)
        .into()
}
