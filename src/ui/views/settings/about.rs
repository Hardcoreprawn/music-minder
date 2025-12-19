//! About section - version info, tagline, credits.

use iced::widget::{Space, column, container, row, text};
use iced::{Alignment, Element, Length};

use crate::ui::icons;
use crate::ui::messages::Message;
use crate::ui::theme::{color, radius, spacing, typography};

use super::section_header;

/// App version from Cargo.toml
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Whimsical taglines (Winamp-inspired personality)
const TAGLINES: &[&str] = &[
    "It really whips the llama's ass!",
    "Native. Fast. No Electron required.",
    "Winamp's spirit, Rust's power.",
    "Your music, your machine.",
    "Built with love and unsafe blocks.",
    "0% JavaScript, 100% audio.",
    "Plays well with others (and llamas).",
];

/// About section with version and credits
pub fn about_section() -> Element<'static, Message> {
    // Pick a tagline based on... something deterministic but fun
    // Using version string hash for now (changes each release)
    let tagline_idx = VERSION.bytes().map(|b| b as usize).sum::<usize>() % TAGLINES.len();
    let tagline = TAGLINES[tagline_idx];

    column![
        section_header(icons::INFO, "About"),
        Space::with_height(spacing::SM),
        // App name and version
        app_info_card(tagline),
        Space::with_height(spacing::MD),
        // Credits
        credits_section(),
    ]
    .spacing(spacing::XS)
    .into()
}

/// Main app info card with logo, name, version, tagline
fn app_info_card(tagline: &str) -> Element<'_, Message> {
    container(
        column![
            // App icon and name
            row![
                text(icons::MUSIC_NOTE).size(32.0).color(color::PRIMARY),
                Space::with_width(spacing::MD),
                column![
                    text("Music Minder")
                        .size(typography::SIZE_TITLE)
                        .color(color::TEXT_PRIMARY),
                    text(format!("Version {}", VERSION))
                        .size(typography::SIZE_SMALL)
                        .color(color::TEXT_MUTED),
                ]
                .spacing(2),
            ]
            .align_y(Alignment::Center),
            Space::with_height(spacing::MD),
            // Tagline
            container(
                text(tagline)
                    .size(typography::SIZE_BODY)
                    .color(color::TEXT_SECONDARY)
            )
            .padding([spacing::SM, spacing::MD])
            .width(Length::Fill)
            .style(|_| container::Style {
                background: Some(color::SURFACE.into()),
                border: iced::Border {
                    color: color::BORDER,
                    width: 1.0,
                    radius: radius::SM.into(),
                },
                ..Default::default()
            }),
        ]
        .spacing(spacing::XS),
    )
    .padding(spacing::MD)
    .width(Length::Fill)
    .style(|_| container::Style {
        background: Some(color::SURFACE_ELEVATED.into()),
        border: iced::Border {
            color: color::BORDER,
            width: 1.0,
            radius: radius::MD.into(),
        },
        ..Default::default()
    })
    .into()
}

/// Credits and acknowledgments
fn credits_section() -> Element<'static, Message> {
    column![
        text("Built With")
            .size(typography::SIZE_SMALL)
            .color(color::TEXT_MUTED),
        Space::with_height(spacing::XS),
        credit_row(icons::GEAR, "Iced", "Cross-platform GUI framework"),
        credit_row(icons::MUSIC, "Symphonia", "Pure Rust audio decoding"),
        credit_row(icons::WAND, "AcoustID", "Audio fingerprinting"),
        credit_row(icons::DISC, "SQLite", "Local library database"),
    ]
    .spacing(spacing::XS)
    .into()
}

/// Single credit row
fn credit_row<'a>(icon: char, name: &'a str, desc: &'a str) -> Element<'a, Message> {
    row![
        text(icon)
            .size(typography::SIZE_SMALL)
            .color(color::TEXT_MUTED),
        Space::with_width(spacing::SM),
        text(name)
            .size(typography::SIZE_SMALL)
            .color(color::TEXT_PRIMARY),
        Space::with_width(spacing::XS),
        text("—")
            .size(typography::SIZE_SMALL)
            .color(color::TEXT_MUTED),
        Space::with_width(spacing::XS),
        text(desc)
            .size(typography::SIZE_SMALL)
            .color(color::TEXT_SECONDARY),
    ]
    .align_y(Alignment::Center)
    .into()
}
