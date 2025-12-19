//! Status indicators section for the enrich pane.

use iced::widget::{Space, container, row, text};
use iced::{Element, Length};

use crate::ui::icons::{self, icon_sized};
use crate::ui::messages::Message;
use crate::ui::state::{EnrichmentPaneState, RateLimitStatus};
use crate::ui::theme::{color, spacing, typography};

/// Status section showing fpcalc, API key, and rate limit status
pub fn status_section(enrich: &EnrichmentPaneState) -> Element<'_, Message> {
    // fpcalc status
    let fpcalc_status = status_indicator(
        "fpcalc",
        enrich.fpcalc_available,
        if enrich.fpcalc_available {
            "ready"
        } else {
            "not found"
        },
    );

    // API key status
    let api_key_status = status_indicator(
        "API key",
        !enrich.api_key.is_empty(),
        if enrich.api_key.is_empty() {
            "not configured"
        } else {
            "configured"
        },
    );

    // Rate limit status
    let rate_status = rate_limit_indicator(&enrich.rate_limit_status);

    container(
        row![
            text("STATUS")
                .size(typography::SIZE_TINY)
                .color(color::TEXT_MUTED),
            Space::with_width(spacing::LG),
            fpcalc_status,
            Space::with_width(spacing::LG),
            api_key_status,
            Space::with_width(spacing::LG),
            rate_status,
        ]
        .align_y(iced::Alignment::Center),
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

/// Single status indicator with icon and label
fn status_indicator<'a>(label: &'a str, ok: bool, detail: &'a str) -> Element<'a, Message> {
    let (icon, icon_color) = if ok {
        (icons::CIRCLE_CHECK, color::SUCCESS)
    } else {
        (icons::CIRCLE_XMARK, color::ERROR)
    };

    row![
        icon_sized(icon, typography::SIZE_SMALL).color(icon_color),
        Space::with_width(spacing::XS),
        text(label)
            .size(typography::SIZE_SMALL)
            .color(color::TEXT_SECONDARY),
        Space::with_width(spacing::XS),
        text(detail)
            .size(typography::SIZE_TINY)
            .color(color::TEXT_MUTED),
    ]
    .align_y(iced::Alignment::Center)
    .into()
}

/// Rate limit status indicator
fn rate_limit_indicator(status: &RateLimitStatus) -> Element<'_, Message> {
    let (icon, icon_color, label) = match status {
        RateLimitStatus::Ok => (icons::CIRCLE_CHECK, color::SUCCESS, "Rate: OK"),
        RateLimitStatus::Warning => (icons::CIRCLE_EXCLAIM, color::WARNING, "Rate: Slow"),
        RateLimitStatus::Limited => (icons::CIRCLE_XMARK, color::ERROR, "Rate Limited"),
    };

    row![
        icon_sized(icon, typography::SIZE_SMALL).color(icon_color),
        Space::with_width(spacing::XS),
        text(label)
            .size(typography::SIZE_SMALL)
            .color(color::TEXT_SECONDARY),
    ]
    .align_y(iced::Alignment::Center)
    .into()
}
