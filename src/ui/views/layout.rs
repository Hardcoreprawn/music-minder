//! Layout composition and main pane structure.

use iced::widget::{Space, button, column, container, row, text};
use iced::{Element, Length};

use crate::ui::canvas::visualization_view;
use crate::ui::icons::{self, icon_sized};
use crate::ui::messages::Message;
use crate::ui::state::{ActivePane, LoadedState};

use super::diagnostics_view::diagnostics_pane;
use super::library::library_pane;
use super::player::player_controls;

/// Main loaded state view - integrated layout with sidebar
pub fn loaded_view(s: &LoadedState) -> Element<'_, Message> {
    let sidebar = sidebar_view(s);
    let main_content = match s.active_pane {
        ActivePane::Library => library_pane(s),
        ActivePane::NowPlaying => now_playing_pane(s),
        ActivePane::Settings => settings_pane(s),
        ActivePane::Diagnostics => diagnostics_pane(s),
    };

    // Player controls always visible at bottom
    let player_bar = player_controls(s);

    column![
        row![
            sidebar,
            container(main_content)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(20),
        ]
        .height(Length::Fill),
        player_bar,
    ]
    .spacing(0)
    .into()
}

/// Sidebar with navigation and status
fn sidebar_view(s: &LoadedState) -> Element<'_, Message> {
    let is_library = s.active_pane == ActivePane::Library;
    let is_playing = s.active_pane == ActivePane::NowPlaying;
    let is_settings = s.active_pane == ActivePane::Settings;
    let is_diagnostics = s.active_pane == ActivePane::Diagnostics;

    // System status indicator
    let system_status = if let Some(ref diag) = s.diagnostics {
        let (status_icon, color) = match diag.overall_rating {
            crate::diagnostics::AudioReadiness::Excellent => (icons::CHECK_CIRCLE, [0.2, 0.7, 0.2]),
            crate::diagnostics::AudioReadiness::Good => (icons::CHECK_CIRCLE, [0.4, 0.7, 0.2]),
            crate::diagnostics::AudioReadiness::Fair => {
                (icons::EXCLAMATION_CIRCLE, [0.8, 0.6, 0.0])
            }
            crate::diagnostics::AudioReadiness::Poor => (icons::X_CIRCLE, [0.8, 0.2, 0.2]),
        };
        row![
            icon_sized(status_icon, 14).color(color),
            text(format!("{:?}", diag.overall_rating))
                .size(12)
                .color(color),
        ]
        .spacing(5)
    } else {
        row![
            icon_sized(icons::INFO_CIRCLE, 14).color([0.5, 0.5, 0.5]),
            text("Checking...").size(12).color([0.5, 0.5, 0.5]),
        ]
        .spacing(5)
    };

    let library_style = if is_library {
        button::primary
    } else {
        button::secondary
    };
    let playing_style = if is_playing {
        button::primary
    } else {
        button::secondary
    };
    let settings_style = if is_settings {
        button::primary
    } else {
        button::secondary
    };
    let diagnostics_style = if is_diagnostics {
        button::primary
    } else {
        button::secondary
    };

    container(
        column![
            text("Music Minder").size(20),
            Space::with_height(20),
            button(
                row![icon_sized(icons::COLLECTION, 14), text(" Library").size(14)]
                    .align_y(iced::Alignment::Center)
            )
            .padding([8, 16])
            .width(Length::Fill)
            .style(library_style)
            .on_press(Message::SwitchPane(ActivePane::Library)),
            button(
                row![
                    icon_sized(icons::MUSIC_NOTE, 14),
                    text(" Now Playing").size(14)
                ]
                .align_y(iced::Alignment::Center)
            )
            .padding([8, 16])
            .width(Length::Fill)
            .style(playing_style)
            .on_press(Message::SwitchPane(ActivePane::NowPlaying)),
            button(
                row![icon_sized(icons::GEAR, 14), text(" Settings").size(14)]
                    .align_y(iced::Alignment::Center)
            )
            .padding([8, 16])
            .width(Length::Fill)
            .style(settings_style)
            .on_press(Message::SwitchPane(ActivePane::Settings)),
            button(
                row![icon_sized(icons::GEAR, 14), text(" Diagnostics").size(14)]
                    .align_y(iced::Alignment::Center)
            )
            .padding([8, 16])
            .width(Length::Fill)
            .style(diagnostics_style)
            .on_press(Message::SwitchPane(ActivePane::Diagnostics)),
            Space::with_height(Length::Fill),
            text("System Status").size(12).color([0.6, 0.6, 0.6]),
            system_status,
            Space::with_height(10),
            text(&s.status_message).size(10).color([0.5, 0.5, 0.5]),
        ]
        .spacing(5)
        .width(Length::Fixed(180.0)),
    )
    .style(|_| container::Style {
        background: Some(iced::Background::Color([0.15, 0.15, 0.18].into())),
        ..Default::default()
    })
    .padding(15)
    .height(Length::Fill)
    .into()
}

/// Now Playing pane with large visualization
fn now_playing_pane(s: &LoadedState) -> Element<'_, Message> {
    use crate::ui::state::VisualizationMode;
    use iced::widget::scrollable;

    let state = &s.player_state;

    // Current track info - use metadata if available
    let (track_name, artist_name) = if let Some(track) = s.current_track_info() {
        (track.title.clone(), track.artist_name.clone())
    } else if let Some(ref path) = state.current_track {
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        (name, String::new())
    } else {
        ("No Track Playing".to_string(), String::new())
    };

    let track_display = if state.current_track.is_some() {
        container(
            column![
                text(track_name).size(32),
                text(artist_name).size(20).color([0.7, 0.7, 0.7]),
                text(state.format_info()).size(14).color([0.5, 0.5, 0.5]),
            ]
            .spacing(5),
        )
        .padding(10)
    } else {
        container(
            column![
                text("No Track Playing").size(32).color([0.4, 0.4, 0.4]),
                text("Select a track from the library to start playing")
                    .size(14)
                    .color([0.4, 0.4, 0.4]),
            ]
            .spacing(5),
        )
        .padding(10)
    };

    // Visualization mode selector - styled buttons
    let viz_buttons = row![
        viz_mode_button(
            "▊ Spectrum",
            VisualizationMode::Spectrum,
            s.visualization_mode
        ),
        viz_mode_button(
            "〜 Waveform",
            VisualizationMode::Waveform,
            s.visualization_mode
        ),
        viz_mode_button(
            "▌ VU Meter",
            VisualizationMode::VuMeter,
            s.visualization_mode
        ),
        viz_mode_button("○ Off", VisualizationMode::Off, s.visualization_mode),
    ]
    .spacing(8);

    // Large visualization canvas
    let viz_height = 300.0;
    let viz_canvas = visualization_view(s.visualization_mode, &s.visualization, viz_height);

    // Queue display
    let queue_section = {
        let queue_header = row![
            text("Play Queue").size(16),
            Space::with_width(Length::Fill),
            text(format!(
                "{} tracks",
                s.player
                    .as_ref()
                    .map(|p| p.queue().items().len())
                    .unwrap_or(0)
            ))
            .size(12)
            .color([0.5, 0.5, 0.5]),
        ];

        let queue_list = if let Some(ref player) = s.player {
            let items: Vec<_> = player
                .queue()
                .items()
                .iter()
                .enumerate()
                .take(10)
                .map(|(i, item)| {
                    let is_current = player.queue().current_index() == Some(i);
                    let bg = if is_current {
                        [0.2, 0.3, 0.4]
                    } else {
                        [0.12, 0.12, 0.15]
                    };
                    let fg = if is_current {
                        [0.4, 0.8, 1.0]
                    } else {
                        [0.7, 0.7, 0.7]
                    };

                    let display_text = if let Some(track) = s.track_info_by_path(&item.path) {
                        format!("{} - {}", track.artist_name, track.title)
                    } else {
                        item.path
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| "Unknown".to_string())
                    };

                    container(row![
                        text(if is_current { ">" } else { " " }).size(12).color(fg),
                        Space::with_width(8),
                        text(display_text).size(12).color(fg),
                    ])
                    .style(move |_| container::Style {
                        background: Some(iced::Background::Color(bg.into())),
                        ..Default::default()
                    })
                    .padding([4, 8])
                    .width(Length::Fill)
                    .into()
                })
                .collect();

            if items.is_empty() {
                column![
                    container(
                        text("Queue is empty - add tracks from the Library")
                            .size(12)
                            .color([0.4, 0.4, 0.4])
                    )
                    .padding(20)
                    .center_x(Length::Fill)
                ]
            } else {
                column(items).spacing(2)
            }
        } else {
            column![
                text("Player not initialized")
                    .size(12)
                    .color([0.6, 0.3, 0.3])
            ]
        };

        column![
            queue_header,
            Space::with_height(8),
            scrollable(queue_list).height(Length::Fixed(150.0)),
        ]
    };

    column![
        track_display,
        Space::with_height(15),
        viz_buttons,
        Space::with_height(10),
        viz_canvas,
        Space::with_height(20),
        queue_section,
    ]
    .spacing(0)
    .into()
}

fn viz_mode_button(
    label: &str,
    mode: crate::ui::state::VisualizationMode,
    current: crate::ui::state::VisualizationMode,
) -> Element<'_, Message> {
    let style = if mode == current {
        button::primary
    } else {
        button::secondary
    };
    button(text(label).size(12))
        .padding([8, 16])
        .style(style)
        .on_press(Message::PlayerVisualizationModeChanged(mode))
        .into()
}

/// Settings pane with file organization and track identification
fn settings_pane(s: &LoadedState) -> Element<'_, Message> {
    use super::library::{enrichment_section, organize_section};

    let dest_path = s.organize_destination.display().to_string();

    column![
        text("Settings").size(28),
        Space::with_height(20),
        // Organize Files section
        text("File Organization").size(20),
        organize_section(s, dest_path),
        Space::with_height(20),
        // Track Identification section
        enrichment_section(s),
    ]
    .spacing(10)
    .into()
}
