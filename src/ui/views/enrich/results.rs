//! Results section for the enrich pane.

use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Element, Length};

use crate::ui::icons::{self, icon_sized};
use crate::ui::messages::Message;
use crate::ui::state::{EnrichmentPaneState, EnrichmentResult, ResultStatus};
use crate::ui::theme::{self, color, spacing, typography};

/// Results section showing identification outcomes
pub fn results_section(enrich: &EnrichmentPaneState) -> Element<'_, Message> {
    let header = row![
        text("RESULTS")
            .size(typography::SIZE_TINY)
            .color(color::TEXT_MUTED),
        Space::with_width(Length::Fill),
    ]
    .align_y(iced::Alignment::Center);

    let results_list: Vec<Element<Message>> = enrich
        .results
        .iter()
        .enumerate()
        .map(|(i, result)| result_row(i, result))
        .collect();

    container(
        column![
            header,
            Space::with_height(spacing::SM),
            scrollable(column(results_list).spacing(spacing::XS)).height(Length::FillPortion(1)),
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
    .height(Length::FillPortion(1))
    .into()
}

/// Single result row
fn result_row(index: usize, result: &EnrichmentResult) -> Element<'_, Message> {
    let (status_icon, status_color) = match result.status {
        ResultStatus::Success => (icons::CIRCLE_CHECK, color::SUCCESS),
        ResultStatus::Warning => (icons::CIRCLE_EXCLAIM, color::WARNING),
        ResultStatus::Error => (icons::CIRCLE_XMARK, color::ERROR),
        ResultStatus::Pending => (icons::SPINNER, color::TEXT_MUTED),
    };

    // Confidence bar - simplified to just show percentage
    let confidence_widget: Element<Message> = if let Some(conf) = result.confidence {
        let conf_color = if conf >= 0.9 {
            color::SUCCESS
        } else if conf >= 0.7 {
            color::WARNING
        } else {
            color::ERROR
        };

        text(format!("{:.0}%", conf * 100.0))
            .size(typography::SIZE_SMALL)
            .color(conf_color)
            .into()
    } else {
        Space::new(0, 0).into()
    };

    // Main content
    let title_text = result.title.as_deref().unwrap_or("Unknown");

    let changes_text: Element<Message> = if !result.changes.is_empty() {
        let changes_str = result.changes.join(", ");
        text(changes_str)
            .size(typography::SIZE_TINY)
            .color(color::TEXT_MUTED)
            .into()
    } else {
        Space::new(0, 0).into()
    };

    // Action buttons
    let review_btn = button(
        text("Review")
            .size(typography::SIZE_TINY)
            .color(color::TEXT_SECONDARY),
    )
    .padding([spacing::XS, spacing::SM])
    .style(theme::button_ghost)
    .on_press(Message::EnrichReviewResult(index));

    let write_btn = if result.status == ResultStatus::Success {
        button(
            text("Write")
                .size(typography::SIZE_TINY)
                .color(color::PRIMARY),
        )
        .padding([spacing::XS, spacing::SM])
        .style(theme::button_ghost)
        .on_press(Message::EnrichWriteResult(index))
    } else {
        button(
            text("Write")
                .size(typography::SIZE_TINY)
                .color(color::TEXT_MUTED),
        )
        .padding([spacing::XS, spacing::SM])
        .style(theme::button_ghost)
    };

    container(
        column![
            row![
                icon_sized(status_icon, typography::SIZE_SMALL).color(status_color),
                Space::with_width(spacing::SM),
                text(title_text)
                    .size(typography::SIZE_SMALL)
                    .color(color::TEXT_PRIMARY),
                Space::with_width(Length::Fill),
                confidence_widget,
                Space::with_width(spacing::MD),
                review_btn,
                write_btn,
            ]
            .align_y(iced::Alignment::Center),
            changes_text,
        ]
        .spacing(spacing::XS),
    )
    .padding(spacing::SM)
    .style(|_| container::Style {
        background: Some(iced::Background::Color(color::SURFACE_ELEVATED)),
        border: iced::Border {
            color: color::BORDER_SUBTLE,
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    })
    .into()
}
