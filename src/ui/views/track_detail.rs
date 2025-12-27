//! Track detail modal view.
//!
//! Shows detailed metadata for a single track, with ability to:
//! - See all available metadata fields
//! - Identify which fields are missing/incomplete
//! - Run fingerprint identification
//! - See and apply enrichment results

use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};

use crate::ui::icons::{self, icon_sized, spinner_frame};
use crate::ui::messages::Message;
use crate::ui::state::LoadedState;
use crate::ui::theme::{self, color, radius, spacing, typography};

/// Track detail modal view
pub fn track_detail_modal(s: &LoadedState) -> Option<Element<'_, Message>> {
    let index = s.track_detail.track_index?;
    let track = s.tracks.get(index)?;

    let content = column![
        // Header with close button
        modal_header(track),
        Space::with_height(spacing::MD),
        // Main content in scrollable area
        scrollable(
            column![
                // File info section
                file_info_section(s, track),
                Space::with_height(spacing::MD),
                // Current metadata section
                metadata_section(s, track),
                Space::with_height(spacing::MD),
                // Enrichment section
                enrichment_section(s),
            ]
            .spacing(spacing::SM)
        )
        .height(Length::Fill),
        Space::with_height(spacing::MD),
        // Action buttons
        action_buttons(s),
    ]
    .spacing(spacing::SM)
    .padding(spacing::LG)
    .width(Length::Fill)
    .height(Length::Fill);

    // Modal container with backdrop
    Some(
        container(
            container(content)
                .width(Length::Fixed(600.0))
                .height(Length::Fixed(600.0))
                .style(modal_style),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(backdrop_style)
        .into(),
    )
}

/// Modal header with title and close button
fn modal_header(track: &crate::db::TrackWithMetadata) -> Element<'_, Message> {
    row![
        column![
            text("Track Details")
                .size(typography::SIZE_HEADING)
                .color(color::TEXT_PRIMARY),
            text(&track.title)
                .size(typography::SIZE_SMALL)
                .color(color::TEXT_MUTED),
        ]
        .spacing(2),
        Space::with_width(Length::Fill),
        button(icon_sized(icons::XMARK, typography::SIZE_HEADING).color(color::TEXT_SECONDARY))
            .padding(spacing::XS)
            .style(theme::button_ghost)
            .on_press(Message::TrackDetailClose),
    ]
    .align_y(Alignment::Center)
    .into()
}

/// File information section
fn file_info_section<'a>(
    s: &LoadedState,
    track: &'a crate::db::TrackWithMetadata,
) -> Element<'a, Message> {
    let path = std::path::Path::new(&track.path);
    let filename = path
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default();

    // Use full_metadata for comprehensive file info, fall back to format_info/basic
    if let Some(ref full) = s.track_detail.full_metadata {
        let format_str = if full.format.is_empty() {
            "Unknown".to_string()
        } else {
            full.format.clone()
        };

        let bitrate_str = full
            .bitrate
            .map(|br| format!("{} kbps", br))
            .unwrap_or_else(|| "—".to_string());

        let sample_rate_str = full
            .sample_rate
            .map(|sr| format!("{} Hz", sr))
            .unwrap_or_else(|| "—".to_string());

        let channels_str = full
            .channels
            .map(|ch| {
                if ch == 1 {
                    "Mono".to_string()
                } else if ch == 2 {
                    "Stereo".to_string()
                } else {
                    format!("{} channels", ch)
                }
            })
            .unwrap_or_else(|| "—".to_string());

        let bits_str = full
            .bits_per_sample
            .map(|b| format!("{}-bit", b))
            .unwrap_or_else(|| "—".to_string());

        let duration_str = {
            let mins = full.duration_secs as u32 / 60;
            let secs = full.duration_secs as u32 % 60;
            format!("{}:{:02}", mins, secs)
        };

        let file_size_str = if full.file_size >= 1_000_000 {
            format!("{:.1} MB", full.file_size as f64 / 1_000_000.0)
        } else {
            format!("{:.1} KB", full.file_size as f64 / 1_000.0)
        };

        let cover_art_str = if full.has_cover_art {
            "Yes ✓".to_string()
        } else {
            "No".to_string()
        };

        section_container(
            "File Information",
            icons::FOLDER,
            column![
                info_row_owned("Filename", filename),
                info_row_owned("Format", format_str),
                info_row_owned("Duration", duration_str),
                info_row_owned("Bitrate", bitrate_str),
                info_row_owned("Sample Rate", sample_rate_str),
                info_row_owned("Channels", channels_str),
                info_row_owned("Bit Depth", bits_str),
                info_row_owned("File Size", file_size_str),
                info_row_owned("Cover Art", cover_art_str),
                info_row("Path", &track.path),
            ]
            .spacing(spacing::XS),
        )
    } else {
        // Fallback to basic info
        let extension = path
            .extension()
            .map(|e| e.to_string_lossy().to_uppercase())
            .unwrap_or_default();

        let duration_str = track
            .duration
            .map(|d| format!("{}:{:02}", d / 60, d % 60))
            .unwrap_or_else(|| "Unknown".to_string());

        let format_detail = if let Some(ref info) = s.track_detail.format_info {
            let mut details = vec![info.extension.to_uppercase()];
            if let Some(br) = info.bitrate {
                details.push(format!("{}kbps", br));
            }
            if let Some(sr) = info.sample_rate {
                details.push(format!("{}kHz", sr / 1000));
            }
            if let Some(ch) = info.channels {
                details.push(if ch == 1 {
                    "Mono".to_string()
                } else {
                    "Stereo".to_string()
                });
            }
            if info.is_lossless {
                details.push("Lossless".to_string());
            }
            details.join(" • ")
        } else {
            extension
        };

        section_container(
            "File Information",
            icons::FOLDER,
            column![
                info_row_owned("Filename", filename),
                info_row_owned("Format", format_detail),
                info_row_owned("Duration", duration_str),
                info_row("Path", &track.path),
            ]
            .spacing(spacing::XS),
        )
    }
}

/// Current metadata section with gap indicators - shows ALL metadata from file
fn metadata_section(
    s: &LoadedState,
    track: &crate::db::TrackWithMetadata,
) -> Element<'static, Message> {
    // Use full_metadata for comprehensive view if available
    if let Some(ref full) = s.track_detail.full_metadata {
        let title = full.title.clone().unwrap_or_else(|| "—".to_string());
        let artist = full.artist.clone().unwrap_or_else(|| "—".to_string());
        let album = full.album.clone().unwrap_or_else(|| "—".to_string());
        let album_artist = full.album_artist.clone().unwrap_or_else(|| "—".to_string());

        let track_str = match (full.track_number, full.total_tracks) {
            (Some(t), Some(total)) => format!("{}/{}", t, total),
            (Some(t), None) => t.to_string(),
            _ => "—".to_string(),
        };

        let disc_str = match (full.disc_number, full.total_discs) {
            (Some(d), Some(total)) => format!("{}/{}", d, total),
            (Some(d), None) => d.to_string(),
            _ => "—".to_string(),
        };

        let year_str = full
            .year
            .map(|y| y.to_string())
            .unwrap_or_else(|| "—".to_string());

        let genre = full.genre.clone().unwrap_or_else(|| "—".to_string());
        let composer = full.composer.clone().unwrap_or_else(|| "—".to_string());
        let comment = full.comment.clone().unwrap_or_else(|| "—".to_string());

        // Truncate long lyrics for display
        let lyrics_preview = full
            .lyrics
            .as_ref()
            .map(|l| {
                if l.len() > 50 {
                    format!("{}... ({} chars)", &l[..47], l.len())
                } else {
                    l.clone()
                }
            })
            .unwrap_or_else(|| "—".to_string());

        let quality_display = track
            .quality_score
            .map(|q| format!("{}%", q))
            .unwrap_or_else(|| "Not assessed".to_string());

        // MusicBrainz IDs section
        let mb_recording = full
            .musicbrainz_recording_id
            .clone()
            .unwrap_or_else(|| "—".to_string());
        let mb_artist = full
            .musicbrainz_artist_id
            .clone()
            .unwrap_or_else(|| "—".to_string());
        let mb_release = full
            .musicbrainz_release_id
            .clone()
            .unwrap_or_else(|| "—".to_string());
        let mb_release_group = full
            .musicbrainz_release_group_id
            .clone()
            .unwrap_or_else(|| "—".to_string());
        let mb_track = full
            .musicbrainz_track_id
            .clone()
            .unwrap_or_else(|| "—".to_string());

        // Build the section with all fields
        section_container(
            "Current Metadata (from file)",
            icons::MUSIC,
            column![
                // Basic info
                text("Basic")
                    .size(typography::SIZE_SMALL)
                    .color(color::TEXT_MUTED),
                metadata_row_owned("Title", title.clone(), full.title.is_none()),
                metadata_row_owned("Artist", artist.clone(), full.artist.is_none()),
                metadata_row_owned("Album", album.clone(), full.album.is_none()),
                metadata_row_owned(
                    "Album Artist",
                    album_artist.clone(),
                    full.album_artist.is_none()
                ),
                Space::with_height(spacing::XS),
                // Track info
                text("Track Info")
                    .size(typography::SIZE_SMALL)
                    .color(color::TEXT_MUTED),
                metadata_row_owned("Track #", track_str, full.track_number.is_none()),
                metadata_row_owned("Disc #", disc_str, full.disc_number.is_none()),
                metadata_row_owned("Year", year_str, full.year.is_none()),
                metadata_row_owned("Genre", genre.clone(), full.genre.is_none()),
                Space::with_height(spacing::XS),
                // Additional info
                text("Additional")
                    .size(typography::SIZE_SMALL)
                    .color(color::TEXT_MUTED),
                metadata_row_owned("Composer", composer.clone(), full.composer.is_none()),
                metadata_row_owned("Comment", comment.clone(), full.comment.is_none()),
                metadata_row_owned("Lyrics", lyrics_preview, full.lyrics.is_none()),
                Space::with_height(spacing::XS),
                // MusicBrainz IDs
                text("MusicBrainz IDs")
                    .size(typography::SIZE_SMALL)
                    .color(color::TEXT_MUTED),
                metadata_row_owned(
                    "Recording ID",
                    truncate_id(&mb_recording),
                    full.musicbrainz_recording_id.is_none()
                ),
                metadata_row_owned(
                    "Artist ID",
                    truncate_id(&mb_artist),
                    full.musicbrainz_artist_id.is_none()
                ),
                metadata_row_owned(
                    "Release ID",
                    truncate_id(&mb_release),
                    full.musicbrainz_release_id.is_none()
                ),
                metadata_row_owned(
                    "Rel. Group ID",
                    truncate_id(&mb_release_group),
                    full.musicbrainz_release_group_id.is_none()
                ),
                metadata_row_owned(
                    "Track ID",
                    truncate_id(&mb_track),
                    full.musicbrainz_track_id.is_none()
                ),
                Space::with_height(spacing::XS),
                // Quality
                text("Quality")
                    .size(typography::SIZE_SMALL)
                    .color(color::TEXT_MUTED),
                metadata_row_owned("Score", quality_display, track.quality_score.is_none()),
            ]
            .spacing(2),
        )
    } else {
        // Fallback to basic metadata from file_metadata or DB
        let (title, artist, album, track_num) = if let Some(ref meta) = s.track_detail.file_metadata
        {
            (
                meta.title.clone(),
                meta.artist.clone(),
                meta.album.clone(),
                meta.track_number,
            )
        } else {
            (
                track.title.clone(),
                track.artist_name.clone(),
                track.album_name.clone(),
                track.track_number.map(|n| n as u32),
            )
        };

        let track_num_str = track_num
            .map(|n| n.to_string())
            .unwrap_or_else(|| "—".to_string());

        let year_str = track
            .year
            .map(|y| y.to_string())
            .unwrap_or_else(|| "—".to_string());

        let quality_display = track
            .quality_score
            .map(|q| format!("{}%", q))
            .unwrap_or_else(|| "Not assessed".to_string());

        section_container(
            "Current Metadata",
            icons::MUSIC,
            column![
                metadata_row_owned("Title", title.clone(), is_unknown(&title)),
                metadata_row_owned("Artist", artist.clone(), is_unknown(&artist)),
                metadata_row_owned("Album", album.clone(), is_unknown(&album)),
                metadata_row_owned("Track #", track_num_str, track_num.is_none()),
                metadata_row_owned("Year", year_str, track.year.is_none()),
                metadata_row_owned("Quality", quality_display, track.quality_score.is_none()),
            ]
            .spacing(spacing::XS),
        )
    }
}

/// Truncate a UUID-style ID for display
fn truncate_id(id: &str) -> String {
    if id == "—" {
        id.to_string()
    } else if id.len() > 20 {
        format!("{}...", &id[..17])
    } else {
        id.to_string()
    }
}

/// Enrichment results section
fn enrichment_section(s: &LoadedState) -> Element<'_, Message> {
    let content: Element<'_, Message> = if s.track_detail.is_identifying {
        // Show spinner while identifying
        container(
            row![
                text(spinner_frame(s.animation_tick))
                    .size(typography::SIZE_HEADING)
                    .color(color::PRIMARY),
                Space::with_width(spacing::SM),
                text("Identifying track...")
                    .size(typography::SIZE_BODY)
                    .color(color::TEXT_SECONDARY),
            ]
            .align_y(Alignment::Center),
        )
        .padding(spacing::MD)
        .width(Length::Fill)
        .center_x(Length::Fill)
        .into()
    } else if let Some(ref id) = s.track_detail.identification {
        // Show identification results with diff
        let confidence_pct = (id.score * 100.0) as u32;
        let confidence_color = if confidence_pct >= 90 {
            color::SUCCESS
        } else if confidence_pct >= 70 {
            color::WARNING
        } else {
            color::ERROR
        };

        // Format disc info
        let disc_str = match (id.track.disc_number, id.track.total_discs) {
            (Some(d), Some(t)) => Some(format!("{}/{}", d, t)),
            (Some(d), None) => Some(d.to_string()),
            _ => None,
        };

        // Format genres (top 3)
        let genres_str = if !id.track.genres.is_empty() {
            Some(
                id.track
                    .genres
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        } else {
            None
        };

        // Format release type
        let release_type = id.track.release_type.as_deref();

        column![
            // Confidence indicator
            row![
                text(format!("{}% confidence", confidence_pct))
                    .size(typography::SIZE_BODY)
                    .color(confidence_color),
                Space::with_width(Length::Fill),
                if s.track_detail.tags_written {
                    Element::from(
                        row![
                            icon_sized(icons::CIRCLE_CHECK, typography::SIZE_SMALL)
                                .color(color::SUCCESS),
                            Space::with_width(spacing::XS),
                            text("Tags written!")
                                .size(typography::SIZE_SMALL)
                                .color(color::SUCCESS),
                        ]
                        .align_y(Alignment::Center),
                    )
                } else {
                    Element::from(Space::with_width(0))
                },
            ]
            .align_y(Alignment::Center),
            Space::with_height(spacing::SM),
            // Show identified values as a diff
            diff_row("Title", id.track.title.as_deref()),
            diff_row("Artist", id.track.artist.as_deref()),
            diff_row("Album", id.track.album.as_deref()),
            diff_row_owned("Track #", id.track.track_number.map(|n| n.to_string())),
            diff_row_owned("Year", id.track.year.map(|y| y.to_string())),
            diff_row_owned("Disc", disc_str),
            diff_row("Type", release_type),
            diff_row_owned("Genres", genres_str),
            // MusicBrainz IDs section
            if id.track.recording_id.is_some() {
                Element::from(
                    column![
                        Space::with_height(spacing::XS),
                        row![
                            icon_sized(icons::CIRCLE_CHECK, typography::SIZE_SMALL)
                                .color(color::SUCCESS),
                            Space::with_width(spacing::XS),
                            text("MusicBrainz IDs")
                                .size(typography::SIZE_SMALL)
                                .color(color::TEXT_MUTED),
                        ]
                        .align_y(Alignment::Center),
                        mb_id_row("Recording", id.track.recording_id.as_deref()),
                        mb_id_row("Artist", id.track.artist_id.as_deref()),
                        mb_id_row("Release", id.track.release_id.as_deref()),
                    ]
                    .spacing(2),
                )
            } else {
                Element::from(Space::with_height(0))
            },
        ]
        .spacing(spacing::XS)
        .into()
    } else if let Some(ref err) = s.track_detail.error {
        // Show error message
        container(
            row![
                icon_sized(icons::CIRCLE_EXCLAIM, typography::SIZE_BODY).color(color::ERROR),
                Space::with_width(spacing::SM),
                text(err).size(typography::SIZE_SMALL).color(color::ERROR),
            ]
            .align_y(Alignment::Center),
        )
        .padding(spacing::SM)
        .width(Length::Fill)
        .style(error_container_style)
        .into()
    } else {
        // No identification yet - show prompt
        container(
            column![
                text("Click 'Identify' to find metadata for this track")
                    .size(typography::SIZE_BODY)
                    .color(color::TEXT_MUTED),
                Space::with_height(spacing::XS),
                text("Uses audio fingerprinting to match against the AcoustID database")
                    .size(typography::SIZE_SMALL)
                    .color(color::TEXT_MUTED),
            ]
            .align_x(Alignment::Center),
        )
        .padding(spacing::MD)
        .width(Length::Fill)
        .center_x(Length::Fill)
        .into()
    };

    section_container("Enrichment", icons::WAND, content)
}

/// Action buttons at the bottom
fn action_buttons(s: &LoadedState) -> Element<'_, Message> {
    let can_identify = !s.track_detail.is_identifying
        && !s.enrichment.api_key.is_empty()
        && s.enrichment.fpcalc_available;

    let can_write = s.track_detail.identification.is_some() && !s.track_detail.tags_written;

    row![
        // Identify button
        button(
            row![
                icon_sized(icons::WAND, typography::SIZE_SMALL).color(color::TEXT_PRIMARY),
                Space::with_width(spacing::XS),
                text(if s.track_detail.is_identifying {
                    "Identifying..."
                } else {
                    "Identify"
                })
                .size(typography::SIZE_BODY),
            ]
            .align_y(Alignment::Center)
        )
        .padding([spacing::SM, spacing::MD])
        .style(theme::button_primary)
        .on_press_maybe(can_identify.then_some(Message::TrackDetailIdentify)),
        Space::with_width(spacing::SM),
        // Write tags button
        button(
            row![
                icon_sized(icons::FLOPPY, typography::SIZE_SMALL).color(color::TEXT_PRIMARY),
                Space::with_width(spacing::XS),
                text("Write Tags").size(typography::SIZE_BODY),
            ]
            .align_y(Alignment::Center)
        )
        .padding([spacing::SM, spacing::MD])
        .style(if can_write {
            theme::button_primary
        } else {
            theme::button_ghost
        })
        .on_press_maybe(can_write.then_some(Message::TrackDetailWriteTags)),
        Space::with_width(Length::Fill),
        // Close button
        button(text("Close").size(typography::SIZE_BODY))
            .padding([spacing::SM, spacing::MD])
            .style(theme::button_ghost)
            .on_press(Message::TrackDetailClose),
    ]
    .align_y(Alignment::Center)
    .into()
}

// ============================================================================
// Helper components
// ============================================================================

/// A section with header and content
fn section_container<'a>(
    title: &'a str,
    icon: char,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(
        column![
            row![
                icon_sized(icon, typography::SIZE_BODY).color(color::TEXT_SECONDARY),
                Space::with_width(spacing::XS),
                text(title)
                    .size(typography::SIZE_BODY)
                    .color(color::TEXT_PRIMARY),
            ]
            .align_y(Alignment::Center),
            Space::with_height(spacing::SM),
            content.into(),
        ]
        .spacing(spacing::XS),
    )
    .padding(spacing::MD)
    .width(Length::Fill)
    .style(section_style)
    .into()
}

/// A simple info row (label: value)
fn info_row<'a>(label: &'a str, value: &'a str) -> Element<'a, Message> {
    row![
        text(label)
            .size(typography::SIZE_SMALL)
            .color(color::TEXT_MUTED)
            .width(Length::Fixed(80.0)),
        text(value)
            .size(typography::SIZE_SMALL)
            .color(color::TEXT_SECONDARY),
    ]
    .spacing(spacing::SM)
    .into()
}

/// A simple info row with owned value (for temporary strings)
fn info_row_owned(label: &'static str, value: String) -> Element<'static, Message> {
    row![
        text(label)
            .size(typography::SIZE_SMALL)
            .color(color::TEXT_MUTED)
            .width(Length::Fixed(80.0)),
        text(value)
            .size(typography::SIZE_SMALL)
            .color(color::TEXT_SECONDARY),
    ]
    .spacing(spacing::SM)
    .into()
}

/// A metadata row with gap indicator (owned version for temporary strings)
fn metadata_row_owned(
    label: &'static str,
    value: String,
    is_gap: bool,
) -> Element<'static, Message> {
    let (icon, icon_color) = if is_gap {
        (icons::CIRCLE_EXCLAIM, color::WARNING)
    } else {
        (icons::CIRCLE_CHECK, color::SUCCESS)
    };

    row![
        icon_sized(icon, typography::SIZE_SMALL).color(icon_color),
        Space::with_width(spacing::XS),
        text(label)
            .size(typography::SIZE_SMALL)
            .color(color::TEXT_MUTED)
            .width(Length::Fixed(70.0)),
        text(value).size(typography::SIZE_SMALL).color(if is_gap {
            color::TEXT_MUTED
        } else {
            color::TEXT_PRIMARY
        }),
    ]
    .spacing(spacing::XS)
    .align_y(Alignment::Center)
    .into()
}

/// A diff row showing new value from identification
fn diff_row<'a>(label: &'a str, new_value: Option<&'a str>) -> Element<'a, Message> {
    let Some(value) = new_value else {
        return Space::with_height(0).into();
    };

    row![
        icon_sized(icons::ARROW_ROTATE, typography::SIZE_SMALL).color(color::PRIMARY),
        Space::with_width(spacing::XS),
        text(label)
            .size(typography::SIZE_SMALL)
            .color(color::TEXT_MUTED)
            .width(Length::Fixed(70.0)),
        text(value)
            .size(typography::SIZE_SMALL)
            .color(color::PRIMARY),
    ]
    .spacing(spacing::XS)
    .align_y(Alignment::Center)
    .into()
}

/// A diff row showing new value from identification (owned version)
fn diff_row_owned(label: &'static str, new_value: Option<String>) -> Element<'static, Message> {
    let Some(value) = new_value else {
        return Space::with_height(0).into();
    };

    row![
        icon_sized(icons::ARROW_ROTATE, typography::SIZE_SMALL).color(color::PRIMARY),
        Space::with_width(spacing::XS),
        text(label)
            .size(typography::SIZE_SMALL)
            .color(color::TEXT_MUTED)
            .width(Length::Fixed(70.0)),
        text(value)
            .size(typography::SIZE_SMALL)
            .color(color::PRIMARY),
    ]
    .spacing(spacing::XS)
    .align_y(Alignment::Center)
    .into()
}

/// A row showing a MusicBrainz ID (truncated for display)
fn mb_id_row<'a>(label: &'a str, id: Option<&'a str>) -> Element<'a, Message> {
    let Some(value) = id else {
        return Space::with_height(0).into();
    };

    // Truncate long UUIDs for display
    let display_value = if value.len() > 20 {
        format!("{}...", &value[..17])
    } else {
        value.to_string()
    };

    row![
        Space::with_width(spacing::MD), // Indent
        text(label)
            .size(typography::SIZE_SMALL - 2)
            .color(color::TEXT_MUTED)
            .width(Length::Fixed(60.0)),
        text(display_value)
            .size(typography::SIZE_SMALL - 2)
            .color(color::TEXT_MUTED),
    ]
    .spacing(spacing::XS)
    .into()
}

/// Check if a value looks like an "unknown" placeholder
fn is_unknown(value: &str) -> bool {
    value.is_empty()
        || value.starts_with("Unknown")
        || value == "—"
        || value.to_lowercase() == "untitled"
}

// ============================================================================
// Styles
// ============================================================================

fn modal_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(color::SURFACE_ELEVATED.into()),
        border: iced::Border {
            color: color::BORDER,
            width: 1.0,
            radius: radius::MD.into(),
        },
        shadow: iced::Shadow {
            color: iced::Color::from_rgba(0.0, 0.0, 0.0, 0.5),
            offset: iced::Vector::new(0.0, 4.0),
            blur_radius: 20.0,
        },
        ..Default::default()
    }
}

fn backdrop_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.6).into()),
        ..Default::default()
    }
}

fn section_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(color::SURFACE.into()),
        border: iced::Border {
            color: color::BORDER,
            width: 1.0,
            radius: radius::SM.into(),
        },
        ..Default::default()
    }
}

fn error_container_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(iced::Color::from_rgba(0.8, 0.2, 0.2, 0.1).into()),
        border: iced::Border {
            color: color::ERROR,
            width: 1.0,
            radius: radius::SM.into(),
        },
        ..Default::default()
    }
}
