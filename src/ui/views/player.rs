//! Player controls and related UI components.

use iced::widget::{Space, button, column, container, image, pick_list, row, slider, text};
use iced::{Border, Element, Length};

use crate::player::PlaybackStatus;
use crate::ui::icons::{self, icon_sized};
use crate::ui::messages::Message;
use crate::ui::state::LoadedState;
use crate::ui::theme::{self, color, layout, spacing, typography};

/// Maximum volume level (because this one goes to 11)
const MAX_VOLUME: f32 = 11.0;

/// Maximum characters for device name display (for future use)
#[allow(dead_code)]
const _MAX_DEVICE_NAME_LEN: usize = 20;

/// Truncate a string with ellipsis if too long (for future use)
#[allow(dead_code)]
fn _truncate_with_ellipsis(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len - 1).collect();
        format!("{}…", truncated)
    }
}

/// Format seconds as MM:SS or HH:MM:SS
fn format_duration_secs(secs: f32) -> String {
    let secs = secs as u64;
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    let secs = secs % 60;

    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, mins, secs)
    } else {
        format!("{}:{:02}", mins, secs)
    }
}

/// Player controls bar (always visible at bottom)
pub fn player_controls(s: &LoadedState) -> Element<'_, Message> {
    let state = &s.player_state;

    // =========================================================================
    // LEFT SECTION: Cover Art + Track Info
    // =========================================================================

    // Mini cover art (48x48) with rounded corners and proper clipping
    let cover_size = layout::COVER_ART_SMALL as f32;
    let cover_widget: Element<Message> = if let Some(ref cover) = s.cover_art.current {
        container(
            image(image::Handle::from_bytes(cover.data.clone()))
                .width(Length::Fixed(cover_size))
                .height(Length::Fixed(cover_size))
                .content_fit(iced::ContentFit::Cover),
        )
        .width(Length::Fixed(cover_size))
        .height(Length::Fixed(cover_size))
        .clip(true)
        .style(|_| container::Style {
            border: Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
    } else {
        // Placeholder with music icon
        container(icon_sized(icons::MUSIC, typography::SIZE_HEADING).color(color::TEXT_MUTED))
            .width(Length::Fixed(cover_size))
            .height(Length::Fixed(cover_size))
            .center_x(Length::Fixed(cover_size))
            .center_y(Length::Fixed(cover_size))
            .style(|_| container::Style {
                background: Some(iced::Background::Color(color::SURFACE_ELEVATED)),
                border: Border {
                    color: color::BORDER_SUBTLE,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            })
            .into()
    };

    // Track info - stacked: Title on top, "Artist • Album" below
    let (title, artist_album) = if let Some(track) = s.current_track_info() {
        let artist_album_str = if !track.album_name.is_empty() {
            format!("{} • {}", track.artist_name, track.album_name)
        } else {
            track.artist_name.clone()
        };
        (track.title.clone(), artist_album_str)
    } else if state.current_track.is_some() {
        // Fallback to filename if not in library
        let name = state
            .current_track
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        (name, String::new())
    } else {
        ("No track playing".to_string(), String::new())
    };

    let track_info_col = column![
        text(title)
            .size(typography::SIZE_BODY)
            .color(if state.current_track.is_some() {
                color::TEXT_PRIMARY
            } else {
                color::TEXT_MUTED
            }),
        text(artist_album)
            .size(typography::SIZE_SMALL)
            .color(color::TEXT_SECONDARY),
    ]
    .spacing(2);

    let left_section = row![
        Space::with_width(spacing::SM), // Padding before cover art
        cover_widget,
        Space::with_width(spacing::MD),
        track_info_col,
    ]
    .align_y(iced::Alignment::Center)
    .width(Length::Fixed(240.0)); // Fixed width for cover + track info

    // =========================================================================
    // CENTER SECTION: Transport Controls + Seek Bar
    // =========================================================================

    // Transport buttons (prev, play/pause, next)
    let prev_btn = button(icon_sized(icons::SKIP_BACK, typography::SIZE_BODY))
        .padding([spacing::SM, spacing::MD])
        .style(theme::button_ghost)
        .on_press(Message::PlayerPrevious);

    let play_btn = match state.status {
        PlaybackStatus::Playing => button(icon_sized(icons::PAUSE, typography::SIZE_HEADING))
            .padding([spacing::SM, spacing::LG])
            .style(theme::button_primary)
            .on_press(Message::PlayerPause),
        _ => button(icon_sized(icons::PLAY, typography::SIZE_HEADING))
            .padding([spacing::SM, spacing::LG])
            .style(theme::button_primary)
            .on_press(Message::PlayerPlay),
    };

    let next_btn = button(icon_sized(icons::SKIP_FORWARD, typography::SIZE_BODY))
        .padding([spacing::SM, spacing::MD])
        .style(theme::button_ghost)
        .on_press(Message::PlayerNext);

    let transport_controls = row![prev_btn, play_btn, next_btn,]
        .spacing(spacing::XS)
        .align_y(iced::Alignment::Center);

    // Time display - show preview position when seeking
    let display_pos = s.seek_preview.unwrap_or_else(|| state.position_fraction());
    let display_time = if s.seek_preview.is_some() {
        let preview_secs = display_pos * state.duration.as_secs_f32();
        format_duration_secs(preview_secs)
    } else {
        state.position_str()
    };

    let time_current = text(display_time)
        .size(typography::SIZE_TINY)
        .color(color::TEXT_SECONDARY);

    let time_total = text(state.duration_str())
        .size(typography::SIZE_TINY)
        .color(color::TEXT_SECONDARY);

    // Seek slider - fills available space
    let seek_slider = slider(0.0..=1.0, display_pos, Message::PlayerSeekPreview)
        .on_release(Message::PlayerSeekRelease)
        .step(0.001)
        .width(Length::Fill)
        .style(theme::slider_style);

    let seek_row = row![
        time_current,
        Space::with_width(spacing::SM),
        seek_slider,
        Space::with_width(spacing::SM),
        time_total,
    ]
    .align_y(iced::Alignment::Center);

    let center_section = column![transport_controls, seek_row,]
        .spacing(spacing::XS)
        .align_x(iced::Alignment::Center)
        .width(Length::Fill); // Stretch to fill available space

    // =========================================================================
    // RIGHT SECTION: Shuffle/Repeat + Volume + Device
    // =========================================================================

    // Shuffle button
    let shuffle_btn = button(icon_sized(icons::SHUFFLE, typography::SIZE_SMALL))
        .padding([spacing::XS, spacing::SM])
        .style(theme::button_ghost)
        .on_press(Message::PlayerShuffleRandom);

    // Repeat button (for now, just visual - can add repeat mode later)
    let repeat_btn = button(icon_sized(icons::REPEAT, typography::SIZE_SMALL))
        .padding([spacing::XS, spacing::SM])
        .style(theme::button_ghost);

    // Volume section
    let volume_display = state.volume * MAX_VOLUME;
    let volume_icon_char = if volume_display < 0.1 {
        icons::VOLUME_MUTE
    } else if volume_display < 5.0 {
        icons::VOLUME_LOW
    } else {
        icons::VOLUME_HIGH
    };

    // Fixed-width container for volume icon to prevent layout shift
    let volume_icon_container = container(
        icon_sized(volume_icon_char, typography::SIZE_SMALL).color(color::TEXT_SECONDARY),
    )
    .width(Length::Fixed(20.0))
    .align_x(iced::alignment::Horizontal::Left);

    let volume_slider = slider(0.0..=MAX_VOLUME, volume_display, |v| {
        Message::PlayerVolumeChanged(v / MAX_VOLUME)
    })
    .step(0.5)
    .width(Length::Fixed(80.0))
    .style(theme::slider_style);

    // Audio device - icon with dropdown picker (fixed width to prevent layout shift)
    // Choose icon based on device name (headphone vs speaker)
    let device_name_lower = s.current_audio_device.to_lowercase();
    let device_icon = if device_name_lower.contains("headphone")
        || device_name_lower.contains("earphone")
        || device_name_lower.contains("headset")
        || device_name_lower.contains("airpod")
        || device_name_lower.contains("buds")
    {
        icons::HEADPHONES
    } else {
        icons::SPEAKER
    };

    // Pick list with fixed width - use full device names, truncation is visual only
    let device_picker = pick_list(
        s.audio_devices.clone(),
        Some(s.current_audio_device.clone()),
        Message::PlayerSelectDevice,
    )
    .text_size(typography::SIZE_SMALL)
    .width(Length::Fixed(150.0))
    .style(theme::pick_list_icon_only);

    // Icon + dropdown in a compact fixed-width container
    let device_section = container(
        row![
            icon_sized(device_icon, typography::SIZE_BODY).color(color::TEXT_SECONDARY),
            device_picker,
        ]
        .spacing(spacing::XS)
        .align_y(iced::Alignment::Center),
    )
    .padding([spacing::XS, spacing::SM])
    .width(Length::Fixed(190.0)) // Fixed width to prevent layout shift
    .style(|_| container::Style {
        background: Some(iced::Background::Color(color::SURFACE_ELEVATED)),
        border: Border {
            color: color::BORDER_SUBTLE,
            width: 1.0,
            radius: 6.0.into(),
        },
        ..Default::default()
    });

    let right_section = row![
        shuffle_btn,
        repeat_btn,
        Space::with_width(spacing::SM),
        volume_icon_container,
        volume_slider,
        Space::with_width(spacing::SM),
        device_section,
    ]
    .spacing(spacing::XS)
    .align_y(iced::Alignment::Center)
    .width(Length::Shrink); // Fixed size, don't stretch or squish

    // =========================================================================
    // ASSEMBLE PLAYER BAR
    // =========================================================================

    container(
        row![left_section, center_section, right_section,]
            .spacing(spacing::LG)
            .align_y(iced::Alignment::Center)
            .padding([spacing::SM, spacing::LG]),
    )
    .style(|_| container::Style {
        background: Some(iced::Background::Color(color::SURFACE)),
        border: Border {
            color: color::BORDER_SUBTLE,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    })
    .width(Length::Fill)
    .height(Length::Fixed(layout::PLAYER_BAR_HEIGHT as f32))
    .into()
}
