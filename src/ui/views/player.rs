//! Player controls and related UI components.

use iced::widget::{Space, button, container, pick_list, row, slider, text};
use iced::{Element, Length};

use crate::player::PlaybackStatus;
use crate::ui::messages::Message;
use crate::ui::state::LoadedState;

/// Maximum volume level (because this one goes to 11)
const MAX_VOLUME: f32 = 11.0;

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

    // Track info - use metadata if available (Artist - Title format)
    let track_info = if let Some(track) = s.current_track_info() {
        text(format!("{} - {}", track.artist_name, track.title)).size(14)
    } else if state.current_track.is_some() {
        // Fallback to filename if not in library
        let name = state
            .current_track
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        text(name).size(14)
    } else {
        text("No track playing").size(14).color([0.5, 0.5, 0.5])
    };

    // Play/pause button - fixed width to prevent layout shift
    let play_btn = match state.status {
        PlaybackStatus::Playing => button(text("||").size(14))
            .padding([8, 10])
            .width(Length::Fixed(40.0))
            .on_press(Message::PlayerPause),
        _ => button(text("|>").size(14))
            .padding([8, 10])
            .width(Length::Fixed(40.0))
            .on_press(Message::PlayerPlay),
    };

    // Time display - show preview position when seeking
    let display_pos = s.seek_preview.unwrap_or_else(|| state.position_fraction());
    let display_time = if s.seek_preview.is_some() {
        // Show preview time while dragging
        let preview_secs = display_pos * state.duration.as_secs_f32();
        format_duration_secs(preview_secs)
    } else {
        state.position_str()
    };
    let time_display = text(format!(
        "{} / {}",
        display_time,
        state.duration_str()
    ))
    .size(12);

    // Seek slider - use on_release to only seek when user finishes dragging
    // on_change updates the preview position (visual feedback)
    // on_release performs the actual seek command using the stored preview position
    let seek_slider = slider(0.0..=1.0, display_pos, Message::PlayerSeekPreview)
        .on_release(Message::PlayerSeekRelease)
        .step(0.001) // Fine-grained seeking
        .width(Length::FillPortion(3));

    // Volume slider: 0-11 scale (because this one goes to 11!)
    // Internal volume is 0.0-1.0, display is 0-11
    let volume_display = state.volume * MAX_VOLUME;
    let volume_slider = slider(0.0..=MAX_VOLUME, volume_display, |v| {
        Message::PlayerVolumeChanged(v / MAX_VOLUME)
    })
    .step(0.5) // Half-step increments
    .width(Length::Fixed(80.0));

    // Audio device picker
    let device_picker = pick_list(
        s.audio_devices.clone(),
        Some(s.current_audio_device.clone()),
        Message::PlayerSelectDevice,
    )
    .width(Length::Fixed(150.0))
    .text_size(11);

    container(
        row![
            button(text("|<").size(14))
                .padding([8, 10])
                .width(Length::Fixed(40.0))
                .on_press(Message::PlayerPrevious),
            play_btn,
            button(text(">|").size(14))
                .padding([8, 10])
                .width(Length::Fixed(40.0))
                .on_press(Message::PlayerNext),
            button(text("Shuffle").size(11))
                .padding([6, 8])
                .on_press(Message::PlayerShuffleRandom),
            Space::with_width(10),
            track_info,
            Space::with_width(10),
            seek_slider,
            Space::with_width(10),
            time_display,
            Space::with_width(15),
            text(format!("{:.0}", volume_display)).size(11),
            volume_slider,
            Space::with_width(10),
            device_picker,
        ]
        .spacing(5)
        .align_y(iced::Alignment::Center)
        .padding(10),
    )
    .style(|_| container::Style {
        background: Some(iced::Background::Color([0.2, 0.2, 0.25].into())),
        ..Default::default()
    })
    .width(Length::Fill)
    .into()
}
