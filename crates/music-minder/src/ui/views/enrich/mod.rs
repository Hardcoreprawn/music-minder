//! Enrich pane - batch metadata identification and tagging.
//!
//! This pane provides:
//! - Status indicators for fpcalc, API key, rate limits
//! - Track selection with checkboxes
//! - Options for fill-only vs overwrite
//! - Progress display during identification
//! - Results list with confidence scores
//! - Batch write actions

mod results;
mod selection;
mod status;

use iced::widget::{Space, button, checkbox, column, container, row, text};
use iced::{Element, Length};

use crate::ui::icons::{self, icon_sized};
use crate::ui::messages::Message;
use crate::ui::state::LoadedState;
use crate::ui::theme::{self, color, spacing, typography};

use results::results_section;
use selection::selection_section;
use status::status_section;

/// Main enrich pane view
pub fn enrich_pane(s: &LoadedState) -> Element<'_, Message> {
    let enrich = &s.enrichment_pane;

    // Header
    let header = text("Enrich Library")
        .size(typography::SIZE_TITLE)
        .color(color::TEXT_PRIMARY);

    // Status section (fpcalc, API key, rate limit)
    let status = status_section(enrich);

    // Track selection section
    let selection = selection_section(s);

    // Options section
    let options = options_section(enrich);

    // Action button
    let can_identify = enrich.fpcalc_available
        && !enrich.api_key.is_empty()
        && !enrich.selected_tracks.is_empty()
        && !enrich.is_identifying;

    let identify_btn = if enrich.is_identifying {
        button(
            row![
                icon_sized(icons::SPINNER, typography::SIZE_BODY).color(color::TEXT_INVERSE),
                text("Identifying...").color(color::TEXT_INVERSE),
            ]
            .spacing(spacing::SM)
            .align_y(iced::Alignment::Center),
        )
        .padding([spacing::SM, spacing::LG])
        .style(theme::button_primary)
    } else {
        let btn = button(
            row![
                icon_sized(icons::WAND, typography::SIZE_BODY).color(color::TEXT_INVERSE),
                text("Identify Selected").color(color::TEXT_INVERSE),
            ]
            .spacing(spacing::SM)
            .align_y(iced::Alignment::Center),
        )
        .padding([spacing::SM, spacing::LG])
        .style(theme::button_primary);

        if can_identify {
            btn.on_press(Message::EnrichBatchIdentify)
        } else {
            btn
        }
    };

    // Progress section (visible during identification)
    let progress = if enrich.is_identifying || !enrich.results.is_empty() {
        progress_section(enrich, s.animation_tick)
    } else {
        Space::new(0, 0).into()
    };

    // Results section
    let results = if !enrich.results.is_empty() {
        results_section(enrich)
    } else {
        Space::new(0, 0).into()
    };

    // Batch actions (visible when we have confirmed results)
    let batch_actions = if enrich.has_confirmed_results() {
        batch_actions_section()
    } else {
        Space::new(0, 0).into()
    };

    column![
        header,
        Space::with_height(spacing::LG),
        status,
        Space::with_height(spacing::LG),
        selection,
        Space::with_height(spacing::MD),
        options,
        Space::with_height(spacing::MD),
        identify_btn,
        Space::with_height(spacing::LG),
        progress,
        results,
        batch_actions,
    ]
    .spacing(0)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// Options section - fill-only, cover art toggles
fn options_section(enrich: &crate::ui::state::EnrichmentPaneState) -> Element<'_, Message> {
    let fill_only_checkbox = checkbox("Fill missing only (safe)", enrich.fill_only)
        .text_size(typography::SIZE_BODY)
        .on_toggle(Message::EnrichFillOnlyToggled);

    let cover_art_checkbox = checkbox("Fetch cover art", enrich.fetch_cover_art)
        .text_size(typography::SIZE_BODY)
        .on_toggle(Message::EnrichFetchCoverArtToggled);

    container(
        column![
            text("OPTIONS")
                .size(typography::SIZE_TINY)
                .color(color::TEXT_MUTED),
            Space::with_height(spacing::SM),
            fill_only_checkbox,
            cover_art_checkbox,
        ]
        .spacing(spacing::XS),
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

/// Progress section with determinate bar and fun messages
fn progress_section(
    enrich: &crate::ui::state::EnrichmentPaneState,
    tick: u32,
) -> Element<'_, Message> {
    use super::loading::LoadingContext;

    let total = enrich.selected_tracks.len();
    let completed = enrich.results.len();
    let progress_pct = if total > 0 {
        (completed as f32 / total as f32).min(1.0)
    } else {
        0.0
    };

    // Use fun message if still identifying, otherwise show completion
    let progress_text = if enrich.is_identifying {
        let fun_msg = LoadingContext::Identifying.message_for_tick(tick);
        format!("{} ({}/{})", fun_msg, completed, total)
    } else {
        format!("Complete: {}/{}", completed, total)
    };

    // Simple progress bar using containers
    let bar_width = 300.0;
    let filled_width = bar_width * progress_pct;

    let progress_bar = container(
        container(Space::new(Length::Fixed(filled_width), Length::Fixed(8.0))).style(|_| {
            container::Style {
                background: Some(iced::Background::Color(color::PRIMARY)),
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        }),
    )
    .width(Length::Fixed(bar_width))
    .height(Length::Fixed(8.0))
    .style(|_| container::Style {
        background: Some(iced::Background::Color(color::SURFACE_ELEVATED)),
        border: iced::Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    });

    row![
        text(progress_text)
            .size(typography::SIZE_SMALL)
            .color(color::TEXT_MUTED),
        Space::with_width(spacing::MD),
        progress_bar,
    ]
    .align_y(iced::Alignment::Center)
    .padding([spacing::SM, 0])
    .into()
}

/// Batch actions - Write All Confirmed, Export Report
fn batch_actions_section() -> Element<'static, Message> {
    let write_all_btn = button(
        row![
            icon_sized(icons::FLOPPY, typography::SIZE_BODY).color(color::TEXT_INVERSE),
            text("Write All Confirmed").color(color::TEXT_INVERSE),
        ]
        .spacing(spacing::SM)
        .align_y(iced::Alignment::Center),
    )
    .padding([spacing::SM, spacing::LG])
    .style(theme::button_primary)
    .on_press(Message::EnrichWriteAllConfirmed);

    let export_btn = button(
        row![
            icon_sized(icons::FILE_EXPORT, typography::SIZE_BODY).color(color::TEXT_SECONDARY),
            text("Export Report").color(color::TEXT_SECONDARY),
        ]
        .spacing(spacing::SM)
        .align_y(iced::Alignment::Center),
    )
    .padding([spacing::SM, spacing::LG])
    .style(theme::button_secondary)
    .on_press(Message::EnrichExportReport);

    container(
        row![write_all_btn, Space::with_width(spacing::MD), export_btn,]
            .align_y(iced::Alignment::Center),
    )
    .padding([spacing::LG, 0])
    .into()
}
