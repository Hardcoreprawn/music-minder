//! Loading state component with fun, randomized messages.
//!
//! Inspired by SimCity's "Reticulating Splines" and Winamp's personality,
//! these loading messages add character to async operations.

// Allow unused variants/functions - they're available for future loading states
#![allow(dead_code)]

use crate::ui::icons::{self, icon_sized, spinner_frame};
use crate::ui::messages::Message;
use crate::ui::theme::{color, spacing, typography};
use iced::widget::{Space, column, container, row, text};
use iced::{Element, Length};

/// Loading message categories for different operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadingContext {
    /// Initial library load
    Library,
    /// Scanning for music files
    Scanning,
    /// Identifying track via AcoustID
    Identifying,
    /// Fetching cover art
    CoverArt,
    /// Writing tags to files
    WritingTags,
    /// Organizing files
    Organizing,
    /// Generic loading
    Generic,
}

impl LoadingContext {
    /// Get serious/informational messages for this context
    fn serious_messages(&self) -> &'static [&'static str] {
        match self {
            LoadingContext::Library => &[
                "Loading library...",
                "Fetching tracks from database...",
                "Building track index...",
            ],
            LoadingContext::Scanning => &[
                "Scanning for audio files...",
                "Reading metadata...",
                "Indexing tracks...",
                "Processing audio files...",
            ],
            LoadingContext::Identifying => &[
                "Generating audio fingerprint...",
                "Querying AcoustID...",
                "Matching track signature...",
                "Analyzing audio...",
            ],
            LoadingContext::CoverArt => &[
                "Fetching cover art...",
                "Searching album artwork...",
                "Downloading cover image...",
            ],
            LoadingContext::WritingTags => &[
                "Writing metadata...",
                "Updating file tags...",
                "Saving changes...",
            ],
            LoadingContext::Organizing => &[
                "Organizing files...",
                "Moving to destination...",
                "Updating file paths...",
            ],
            LoadingContext::Generic => &["Loading...", "Please wait...", "Working on it..."],
        }
    }

    /// Get silly/fun messages (Winamp/SimCity inspired)
    fn silly_messages(&self) -> &'static [&'static str] {
        match self {
            LoadingContext::Library => &[
                "Reticulating splines...",
                "Herding audio llamas...",
                "Whipping the llama's ass...",
                "Dusting off the vinyl collection...",
                "Untangling headphone cables...",
                "Calibrating subwoofers...",
                "Warming up the vacuum tubes...",
                "Rewinding the tapes...",
                "Polishing the laser lens...",
                "Defragmenting the groove...",
            ],
            LoadingContext::Scanning => &[
                "Interrogating your hard drive...",
                "Sniffing out bangers...",
                "Cataloguing your questionable taste...",
                "Finding those hidden MP3s...",
                "Judging your folder structure...",
                "Discovering forgotten gems...",
                "Exhuming buried tracks...",
                "Liberating imprisoned audio...",
            ],
            LoadingContext::Identifying => &[
                "Consulting the oracle...",
                "Asking the music gods...",
                "Reverse-engineering vibes...",
                "Decoding sonic DNA...",
                "Running acoustic forensics...",
                "Matching wavelength signatures...",
                "Summoning metadata spirits...",
                "Consulting the album archives...",
            ],
            LoadingContext::CoverArt => &[
                "Hunting for album art...",
                "Raiding the cover archive...",
                "Finding something pretty...",
                "Locating visual companion...",
                "Retrieving aesthetic data...",
                "Downloading eye candy...",
            ],
            LoadingContext::WritingTags => &[
                "Inscribing ancient metadata...",
                "Tattooing your files...",
                "Embedding secrets...",
                "Branding the bytes...",
                "Chiseling ID3 tags...",
            ],
            LoadingContext::Organizing => &[
                "Marie Kondo-ing your music...",
                "Alphabetizing aggressively...",
                "Tidying the chaos...",
                "Creating order from entropy...",
                "Filing the unfiled...",
                "Sorting with extreme prejudice...",
            ],
            LoadingContext::Generic => &[
                "Doing computer things...",
                "Calculating the vibe...",
                "Processing... beep boop...",
                "Thinking really hard...",
                "Consulting the algorithms...",
            ],
        }
    }

    /// Get a message based on tick (rotates through messages, mixing serious and silly)
    pub fn message_for_tick(&self, tick: u32) -> &'static str {
        let serious = self.serious_messages();
        let silly = self.silly_messages();

        // Change message every ~3 seconds (180 ticks at 60fps)
        let message_idx = (tick / 180) as usize;

        // Alternate: 2 serious, 1 silly pattern
        let combined_len = serious.len() + silly.len();
        let idx = message_idx % combined_len;

        if idx < serious.len() {
            serious[idx]
        } else {
            silly[idx - serious.len()]
        }
    }

    /// Get a random silly message (for one-shot displays)
    pub fn random_silly(&self) -> &'static str {
        let silly = self.silly_messages();
        // Use current time as simple randomness source
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as usize;
        silly[now % silly.len()]
    }
}

/// Renders a loading indicator with spinner and message
pub fn loading_indicator<'a>(
    context: LoadingContext,
    tick: u32,
    detail: Option<&'a str>,
) -> Element<'a, Message> {
    let spinner = spinner_frame(tick);
    let message = context.message_for_tick(tick);

    let content = if let Some(detail) = detail {
        column![
            row![
                container(
                    text(spinner)
                        .size(typography::SIZE_BODY)
                        .color(color::PRIMARY)
                )
                .width(Length::Fixed(24.0))
                .center_x(Length::Fixed(24.0)),
                Space::with_width(spacing::SM),
                text(message)
                    .size(typography::SIZE_BODY)
                    .color(color::TEXT_PRIMARY),
            ]
            .align_y(iced::Alignment::Center),
            Space::with_height(spacing::XS),
            text(detail)
                .size(typography::SIZE_SMALL)
                .color(color::TEXT_MUTED),
        ]
        .align_x(iced::Alignment::Center)
    } else {
        column![
            row![
                container(
                    text(spinner)
                        .size(typography::SIZE_BODY)
                        .color(color::PRIMARY)
                )
                .width(Length::Fixed(24.0))
                .center_x(Length::Fixed(24.0)),
                Space::with_width(spacing::SM),
                text(message)
                    .size(typography::SIZE_BODY)
                    .color(color::TEXT_PRIMARY),
            ]
            .align_y(iced::Alignment::Center),
        ]
        .align_x(iced::Alignment::Center)
    };

    container(content)
        .padding(spacing::MD)
        .center_x(Length::Fill)
        .into()
}

/// Large centered loading state (for full-pane loading)
pub fn loading_state_large<'a>(
    context: LoadingContext,
    tick: u32,
    detail: Option<&'a str>,
) -> Element<'a, Message> {
    let spinner = spinner_frame(tick);
    let message = context.message_for_tick(tick);

    // Icon for context (using available icons)
    let context_icon = match context {
        LoadingContext::Library => icons::MUSIC,
        LoadingContext::Scanning => icons::FOLDER_OPEN,
        LoadingContext::Identifying => icons::WAND, // fingerprint -> wand
        LoadingContext::CoverArt => icons::DISC,    // image -> disc
        LoadingContext::WritingTags => icons::FLOPPY, // tag -> floppy
        LoadingContext::Organizing => icons::FOLDER, // folder_tree -> folder
        LoadingContext::Generic => icons::CLOCK,
    };

    let mut content = column![
        icon_sized(context_icon, 32).color(color::TEXT_MUTED),
        Space::with_height(spacing::MD),
        row![
            container(
                text(spinner)
                    .size(typography::SIZE_HEADING)
                    .color(color::PRIMARY)
            )
            .width(Length::Fixed(32.0))
            .center_x(Length::Fixed(32.0)),
            Space::with_width(spacing::SM),
            text(message)
                .size(typography::SIZE_HEADING)
                .color(color::TEXT_PRIMARY),
        ]
        .align_y(iced::Alignment::Center),
    ]
    .align_x(iced::Alignment::Center)
    .spacing(0);

    if let Some(detail) = detail {
        content = content.push(Space::with_height(spacing::SM));
        content = content.push(
            text(detail)
                .size(typography::SIZE_BODY)
                .color(color::TEXT_MUTED),
        );
    }

    container(content)
        .padding(spacing::XXL)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_contexts_have_messages() {
        let contexts = [
            LoadingContext::Library,
            LoadingContext::Scanning,
            LoadingContext::Identifying,
            LoadingContext::CoverArt,
            LoadingContext::WritingTags,
            LoadingContext::Organizing,
            LoadingContext::Generic,
        ];

        for ctx in contexts {
            assert!(!ctx.serious_messages().is_empty());
            assert!(!ctx.silly_messages().is_empty());
            // Should not panic
            let _ = ctx.message_for_tick(0);
            let _ = ctx.message_for_tick(1000);
            let _ = ctx.random_silly();
        }
    }
}
