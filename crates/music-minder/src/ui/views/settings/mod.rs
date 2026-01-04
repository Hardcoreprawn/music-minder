//! Settings pane - organized sections for app configuration.
//!
//! Sections:
//! - Audio: Device selection, visualization mode
//! - Library: Watch paths, scan settings  
//! - Enrichment: AcoustID API key, fpcalc status
//! - Appearance: Theme settings (future)
//! - About: Version, tagline, credits

mod about;
mod appearance;
mod audio;
mod enrichment;
mod library;

use iced::Element;
use iced::widget::{Space, column, container, row, scrollable, text};

use crate::ui::icons::icon_sized;
use crate::ui::messages::Message;
use crate::ui::state::LoadedState;
use crate::ui::theme::{color, spacing, typography};

pub use about::about_section;
pub use appearance::appearance_section;
pub use audio::audio_section;
pub use enrichment::enrichment_section;
pub use library::library_section;

/// Main settings pane with organized sections
pub fn settings_pane(s: &LoadedState) -> Element<'_, Message> {
    let content = column![
        // Header
        text("Settings")
            .size(typography::SIZE_TITLE)
            .color(color::TEXT_PRIMARY),
        Space::with_height(spacing::LG),
        // Audio section
        audio_section(s),
        section_divider(),
        // Library section
        library_section(s),
        section_divider(),
        // Enrichment section
        enrichment_section(s),
        section_divider(),
        // Appearance section
        appearance_section(s),
        section_divider(),
        // About section
        about_section(),
    ]
    .spacing(spacing::MD)
    .padding(spacing::LG);

    scrollable(container(content).width(iced::Length::Fill)).into()
}

/// Visual divider between sections
fn section_divider() -> Element<'static, Message> {
    container(Space::with_height(1))
        .width(iced::Length::Fill)
        .style(|_theme| container::Style {
            background: Some(color::BORDER.into()),
            ..Default::default()
        })
        .padding([spacing::MD, 0])
        .into()
}

/// Section header with icon and title
pub fn section_header(icon: char, title: &str) -> Element<'_, Message> {
    row![
        icon_sized(icon, typography::SIZE_HEADING).color(color::TEXT_SECONDARY),
        Space::with_width(spacing::SM),
        text(title)
            .size(typography::SIZE_HEADING)
            .color(color::TEXT_PRIMARY),
    ]
    .spacing(spacing::XS)
    .into()
}

/// Label for a setting row
pub fn setting_label(label: &str) -> Element<'_, Message> {
    text(label)
        .size(typography::SIZE_BODY)
        .color(color::TEXT_SECONDARY)
        .into()
}

/// Description text below a setting
pub fn setting_description(desc: &str) -> Element<'_, Message> {
    text(desc)
        .size(typography::SIZE_SMALL)
        .color(color::TEXT_MUTED)
        .into()
}
