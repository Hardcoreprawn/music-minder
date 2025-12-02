//! Player controls and related UI components.

use iced::widget::{Space, button, container, pick_list, row, slider, text};
use iced::{Element, Length};

use crate::player::PlaybackStatus;
use crate::ui::messages::Message;
use crate::ui::state::LoadedState;

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

    // Play/pause button - using simple ASCII
    let play_btn = match state.status {
        PlaybackStatus::Playing => button(text("||").size(14))
            .padding([8, 10])
            .on_press(Message::PlayerPause),
        _ => button(text("|>").size(14))
            .padding([8, 10])
            .on_press(Message::PlayerPlay),
    };

    // Time display
    let time_display = text(format!(
        "{} / {}",
        state.position_str(),
        state.duration_str()
    ))
    .size(12);

    // Seek slider
    let seek_pos = state.position_fraction();
    let seek_slider =
        slider(0.0..=1.0, seek_pos, Message::PlayerSeek).width(Length::FillPortion(3));

    // Volume slider
    let volume_slider =
        slider(0.0..=1.0, state.volume, Message::PlayerVolumeChanged).width(Length::Fixed(80.0));

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
                .on_press(Message::PlayerPrevious),
            play_btn,
            button(text(">|").size(14))
                .padding([8, 10])
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
            text("Vol").size(11),
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
