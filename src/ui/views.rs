//! View rendering functions for the UI components.

use iced::widget::{button, column, container, pick_list, row, scrollable, slider, text, text_input, Space};
use iced::{Element, Length};
use std::path::Path;

use super::canvas::visualization_view;
use super::icons::{self, icon_sized};
use super::messages::Message;
use super::state::{LoadedState, OrganizeView, ActivePane, VisualizationMode, virtualization as virt};
use crate::player::PlaybackStatus;
use crate::diagnostics::CheckStatus;

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
        ].height(Length::Fill),
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
            crate::diagnostics::AudioReadiness::Fair => (icons::EXCLAMATION_CIRCLE, [0.8, 0.6, 0.0]),
            crate::diagnostics::AudioReadiness::Poor => (icons::X_CIRCLE, [0.8, 0.2, 0.2]),
        };
        row![
            icon_sized(status_icon, 14).color(color),
            text(format!("{:?}", diag.overall_rating)).size(12).color(color),
        ].spacing(5)
    } else {
        row![
            icon_sized(icons::INFO_CIRCLE, 14).color([0.5, 0.5, 0.5]),
            text("Checking...").size(12).color([0.5, 0.5, 0.5]),
        ].spacing(5)
    };
    
    let library_style = if is_library { button::primary } else { button::secondary };
    let playing_style = if is_playing { button::primary } else { button::secondary };
    let settings_style = if is_settings { button::primary } else { button::secondary };
    let diagnostics_style = if is_diagnostics { button::primary } else { button::secondary };
    
    container(
        column![
            text("Music Minder").size(20),
            Space::with_height(20),
            button(row![icon_sized(icons::COLLECTION, 14), text(" Library").size(14)].align_y(iced::Alignment::Center))
                .padding([8, 16])
                .width(Length::Fill)
                .style(library_style)
                .on_press(Message::SwitchPane(ActivePane::Library)),
            button(row![icon_sized(icons::MUSIC_NOTE, 14), text(" Now Playing").size(14)].align_y(iced::Alignment::Center))
                .padding([8, 16])
                .width(Length::Fill)
                .style(playing_style)
                .on_press(Message::SwitchPane(ActivePane::NowPlaying)),
            button(row![icon_sized(icons::GEAR, 14), text(" Settings").size(14)].align_y(iced::Alignment::Center))
                .padding([8, 16])
                .width(Length::Fill)
                .style(settings_style)
                .on_press(Message::SwitchPane(ActivePane::Settings)),
            button(row![icon_sized(icons::GEAR, 14), text(" Diagnostics").size(14)].align_y(iced::Alignment::Center))
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
        .width(Length::Fixed(180.0))
    )
    .style(|_| container::Style {
        background: Some(iced::Background::Color([0.15, 0.15, 0.18].into())),
        ..Default::default()
    })
    .padding(15)
    .height(Length::Fill)
    .into()
}

/// Player controls bar (always visible at bottom)
fn player_controls(s: &LoadedState) -> Element<'_, Message> {
    let state = &s.player_state;
    
    // Track info - use metadata if available (Artist - Title format)
    let track_info = if let Some(track) = s.current_track_info() {
        text(format!("{} - {}", track.artist_name, track.title)).size(14)
    } else if state.current_track.is_some() {
        // Fallback to filename if not in library
        let name = state.current_track.as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        text(name).size(14)
    } else {
        text("No track playing").size(14).color([0.5, 0.5, 0.5])
    };
    
    // Play/pause button - using simple ASCII
    let play_btn = match state.status {
        PlaybackStatus::Playing => button(text("||").size(14)).padding([8, 10]).on_press(Message::PlayerPause),
        _ => button(text("|>").size(14)).padding([8, 10]).on_press(Message::PlayerPlay),
    };
    
    // Time display
    let time_display = text(format!(
        "{} / {}",
        state.position_str(),
        state.duration_str()
    )).size(12);
    
    // Seek slider
    let seek_pos = state.position_fraction();
    let seek_slider = slider(0.0..=1.0, seek_pos, Message::PlayerSeek)
        .width(Length::FillPortion(3));
    
    // Volume slider
    let volume_slider = slider(0.0..=1.0, state.volume, Message::PlayerVolumeChanged)
        .width(Length::Fixed(80.0));
    
    // Audio device picker
    let device_picker = pick_list(
        s.audio_devices.clone(),
        Some(s.current_audio_device.clone()),
        Message::PlayerSelectDevice,
    ).width(Length::Fixed(150.0)).text_size(11);
    
    container(
        row![
            button(text("|<").size(14)).padding([8, 10]).on_press(Message::PlayerPrevious),
            play_btn,
            button(text(">|").size(14)).padding([8, 10]).on_press(Message::PlayerNext),
            button(text("Shuffle").size(11)).padding([6, 8]).on_press(Message::PlayerShuffleRandom),
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
        .padding(10)
    )
    .style(|_| container::Style {
        background: Some(iced::Background::Color([0.2, 0.2, 0.25].into())),
        ..Default::default()
    })
    .width(Length::Fill)
    .into()
}

/// Library pane with scanning, organizing, and track list
fn library_pane(s: &LoadedState) -> Element<'_, Message> {
    let scan_path = s.scan_path.display().to_string();

    // Loading indicator for tracks
    let track_count_text = if s.is_scanning {
        text("Loading tracks...").size(16).color([0.6, 0.6, 0.6])
    } else {
        text(format!("{} tracks", s.tracks.len())).size(16)
    };

    column![
        text("Library").size(28),
        scan_controls(s, scan_path),
        track_count_text,
        Space::with_height(10),
        track_list(s),
    ]
    .spacing(10)
    .into()
}

/// Now Playing pane with large visualization
fn now_playing_pane(s: &LoadedState) -> Element<'_, Message> {
    let state = &s.player_state;
    
    // Current track info - use metadata if available
    let (track_name, artist_name) = if let Some(track) = s.current_track_info() {
        (track.title.clone(), track.artist_name.clone())
    } else if let Some(ref path) = state.current_track {
        let name = path.file_stem()
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
            ].spacing(5)
        )
        .padding(10)
    } else {
        container(
            column![
                text("No Track Playing").size(32).color([0.4, 0.4, 0.4]),
                text("Select a track from the library to start playing").size(14).color([0.4, 0.4, 0.4]),
            ].spacing(5)
        )
        .padding(10)
    };
    
    // Visualization mode selector - styled buttons
    let viz_buttons = row![
        viz_mode_button("▊ Spectrum", VisualizationMode::Spectrum, s.visualization_mode),
        viz_mode_button("〜 Waveform", VisualizationMode::Waveform, s.visualization_mode),
        viz_mode_button("▌ VU Meter", VisualizationMode::VuMeter, s.visualization_mode),
        viz_mode_button("○ Off", VisualizationMode::Off, s.visualization_mode),
    ].spacing(8);
    
    // Large visualization canvas - takes up most of the space
    let viz_height = 300.0; // Much larger visualization
    let viz_canvas = visualization_view(s.visualization_mode, &s.visualization, viz_height);
    
    // Queue display - compact at bottom
    let queue_section = {
        let queue_header = row![
            text("Play Queue").size(16),
            Space::with_width(Length::Fill),
            text(format!("{} tracks", s.player.as_ref().map(|p| p.queue().items().len()).unwrap_or(0)))
                .size(12)
                .color([0.5, 0.5, 0.5]),
        ];
        
        let queue_list = if let Some(ref player) = s.player {
            let items: Vec<_> = player.queue().items().iter()
                .enumerate()
                .take(10) // Show max 10 items
                .map(|(i, item)| {
                    let is_current = player.queue().current_index() == Some(i);
                    let bg = if is_current { [0.2, 0.3, 0.4] } else { [0.12, 0.12, 0.15] };
                    let fg = if is_current { [0.4, 0.8, 1.0] } else { [0.7, 0.7, 0.7] };
                    
                    // Look up metadata from tracks list
                    let display_text = if let Some(track) = s.track_info_by_path(&item.path) {
                        format!("{} - {}", track.artist_name, track.title)
                    } else {
                        // Fallback to filename
                        item.path.file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| "Unknown".to_string())
                    };
                    
                    container(
                        row![
                            text(if is_current { ">" } else { " " }).size(12).color(fg),
                            Space::with_width(8),
                            text(display_text).size(12).color(fg),
                        ]
                    )
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
                        text("Queue is empty - add tracks from the Library").size(12).color([0.4, 0.4, 0.4])
                    ).padding(20).center_x(Length::Fill)
                ]
            } else {
                column(items).spacing(2)
            }
        } else {
            column![text("Player not initialized").size(12).color([0.6, 0.3, 0.3])]
        };
        
        column![
            queue_header,
            Space::with_height(8),
            scrollable(queue_list).height(Length::Fixed(150.0)),
        ]
    };
    
    // Main layout - visualization prominent
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

fn viz_mode_button(label: &str, mode: VisualizationMode, current: VisualizationMode) -> Element<'_, Message> {
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

/// Diagnostics pane
fn diagnostics_pane(s: &LoadedState) -> Element<'_, Message> {
    let run_button = if s.diagnostics_loading {
        button("Running...").padding(10)
    } else {
        button("Run Diagnostics").padding(10).on_press(Message::DiagnosticsRunPressed)
    };
    
    let results = if let Some(ref diag) = s.diagnostics {
        let checks: Vec<_> = diag.checks.iter().map(|check| {
            let (check_icon, color) = match check.status {
                CheckStatus::Pass => (icons::CHECK_CIRCLE, [0.2, 0.7, 0.2]),
                CheckStatus::Warning => (icons::EXCLAMATION_TRIANGLE, [0.8, 0.6, 0.0]),
                CheckStatus::Fail => (icons::X_CIRCLE, [0.8, 0.2, 0.2]),
                CheckStatus::Info => (icons::INFO_CIRCLE, [0.3, 0.5, 0.8]),
            };
            
            row![
                icon_sized(check_icon, 16).color(color),
                column![
                    text(&check.name).size(14),
                    text(&check.value).size(12).color([0.5, 0.5, 0.5]),
                ].spacing(2),
            ]
            .spacing(10)
            .into()
        }).collect();
        
        column![
            text(format!("Audio Readiness: {:?}", diag.overall_rating)).size(18),
            Space::with_height(10),
            column(checks).spacing(10),
        ]
    } else {
        column![
            text("No diagnostics run yet").size(14).color([0.5, 0.5, 0.5]),
            text("Click 'Run Diagnostics' to check your system").size(12).color([0.5, 0.5, 0.5]),
        ]
    };
    
    column![
        text("System Diagnostics").size(28),
        Space::with_height(10),
        run_button,
        Space::with_height(20),
        scrollable(results).height(Length::Fill),
    ]
    .spacing(10)
    .into()
}

/// Helper to create a conditionally-enabled button
fn action_button<'a>(label: &'a str, msg: Option<Message>) -> iced::widget::Button<'a, Message> {
    match msg {
        Some(m) => button(label).padding(10).on_press(m),
        None => button(label).padding(10),
    }
}

/// Renders the scan controls row
fn scan_controls(state: &LoadedState, path_display: String) -> Element<'_, Message> {
    let (label, msg) = if state.is_scanning { 
        ("Stop Scan", Message::ScanStopped) 
    } else { 
        ("Scan Library", Message::ScanPressed) 
    };
    row![
        text_input("Path to scan", &path_display).on_input(Message::PathChanged).padding(10).width(Length::FillPortion(3)),
        button("Browse").on_press(Message::PickPath).padding(10),
        button(label).on_press(msg).padding(10),
    ].spacing(10).into()
}

/// Renders the organize section based on current view
fn organize_section(state: &LoadedState, dest: String) -> Element<'_, Message> {
    match &state.organize_view {
        OrganizeView::Input => organize_input(state, dest),
        OrganizeView::Preview => organize_preview(state, dest),
        OrganizeView::Organizing => organize_progress(state),
    }
}

/// Renders the organize input view
fn organize_input(state: &LoadedState, dest: String) -> Element<'_, Message> {
    let undo = if state.can_undo { Some(Message::UndoPressed) } else { None };
    column![
        text("Organize Files").size(20),
        row![
            text_input("Destination folder", &dest).on_input(Message::OrganizeDestinationChanged).padding(10).width(Length::FillPortion(3)),
            button("Browse").on_press(Message::PickOrganizeDestination).padding(10),
        ].spacing(10),
        row![
            text_input("Pattern: {Artist}/{Album}/{TrackNum} - {Title}.{ext}", &state.organize_pattern)
                .on_input(Message::OrganizePatternChanged).padding(10).width(Length::FillPortion(3)),
            button("Preview").on_press(Message::OrganizePreviewPressed).padding(10),
            action_button("Undo Last", undo),
        ].spacing(10),
    ].spacing(10).into()
}

/// Renders the organize preview view
fn organize_preview(state: &LoadedState, dest: String) -> Element<'_, Message> {
    let n = state.organize_preview.len();
    let title = if state.preview_loading { format!("Loading... {} files so far", n) } else { format!("Preview: {} files will be moved", n) };
    let confirm = if state.preview_loading { None } else { Some(Message::OrganizeConfirmPressed) };
    
    let header = column![
        text(title).size(20),
        text(format!("Destination: {}", dest)).size(12).color([0.5, 0.5, 0.5]),
        row![
            button("Cancel").on_press(Message::OrganizeCancelPressed).padding(10),
            Space::with_width(Length::Fill),
            action_button("Organize Files", confirm),
        ].spacing(10),
    ].spacing(10);

    let list: Element<Message> = if n > 0 { virtualized_preview_list(state) } else { text("No files to organize").size(14).into() };
    column![header, list].spacing(10).height(Length::Fill).into()
}

/// Renders the organizing progress view
fn organize_progress(state: &LoadedState) -> Element<'_, Message> {
    let errors = state.organize_errors.len();
    column![
        text(format!("Organizing... {} of {} files", state.organize_progress, state.organize_total)).size(20),
        if errors > 0 { text(format!("{} errors", errors)).size(14).color([0.8, 0.4, 0.0]) } else { text("").size(14) },
    ].spacing(10).into()
}

/// Renders virtualized preview list
fn virtualized_preview_list(state: &LoadedState) -> Element<'_, Message> {
    let (start, end, top, bottom) = calc_visible_range(
        state.preview_scroll_offset, state.preview_viewport_height, 
        state.organize_preview.len(), virt::PREVIEW_ROW_HEIGHT,
    );
    let dest = &state.organize_destination;
    let items: Vec<_> = state.organize_preview[start..end].iter()
        .map(|p| preview_item(p, dest, virt::PREVIEW_ROW_HEIGHT)).collect();

    scrollable(column![
        Space::with_height(Length::Fixed(top)),
        column(items).width(Length::Fill),
        Space::with_height(Length::Fixed(bottom)),
    ].width(Length::Fill))
    .height(Length::Fill).width(Length::Fill).on_scroll(Message::PreviewScrollChanged).into()
}

/// Renders a single preview item
fn preview_item<'a>(p: &'a crate::organizer::OrganizePreview, base: &Path, h: f32) -> Element<'a, Message> {
    let from = p.source.strip_prefix(base).unwrap_or(&p.source).display().to_string();
    let to = p.destination.strip_prefix(base).unwrap_or(&p.destination).display().to_string();
    let same = from == to;
    let txt = if same { format!("{} → (no change)", from) } else { format!("{} → {}", from, to) };
    container(text(txt).size(12).color(if same { [0.5, 0.5, 0.5] } else { [0.2, 0.2, 0.2] }))
        .height(Length::Fixed(h)).width(Length::Fill).into()
}

/// Renders virtualized track list with play buttons
fn track_list(state: &LoadedState) -> Element<'_, Message> {
    let (start, end, top, bottom) = calc_visible_range(
        state.scroll_offset, state.viewport_height, state.tracks.len(), virt::TRACK_ROW_HEIGHT,
    );
    let selected = state.enrichment.selected_track;
    let items = state.tracks[start..end].iter().enumerate().map(|(i, t)| {
        let idx = start + i;
        let is_selected = selected == Some(idx);
        let bg_color = if is_selected { [0.25, 0.35, 0.45] } else { [0.18, 0.18, 0.22] };
        let text_color = if is_selected { [0.9, 0.95, 1.0] } else { [0.85, 0.85, 0.85] };
        
        row![
            // Play button - ASCII
            button(text(">").size(12))
                .padding([4, 8])
                .on_press(Message::PlayerPlayTrack(idx)),
            // Queue button  
            button(text("+").size(14))
                .padding([4, 8])
                .on_press(Message::PlayerQueueTrack(idx)),
            // Track info (clickable for enrichment)
            button(
                container(text(format!("{} - {}", t.title, t.artist_name)).color(text_color))
                    .height(Length::Fixed(virt::TRACK_ROW_HEIGHT))
                    .center_y(Length::Fixed(virt::TRACK_ROW_HEIGHT))
                    .width(Length::Fill)
            )
            .style(move |_theme, _status| button::Style {
                background: Some(iced::Background::Color(bg_color.into())),
                text_color: iced::Color::from_rgb(text_color[0], text_color[1], text_color[2]),
                border: iced::Border::default(),
                shadow: iced::Shadow::default(),
            })
            .padding(0)
            .width(Length::Fill)
            .on_press(Message::EnrichmentTrackSelected(idx)),
        ]
        .spacing(5)
        .into()
    });
    scrollable(column![
        Space::with_height(Length::Fixed(top)),
        column(items).width(Length::Fill),
        Space::with_height(Length::Fixed(bottom)),
    ].width(Length::Fill))
    .height(Length::Fill).width(Length::Fill).on_scroll(Message::ScrollChanged).into()
}

/// Calculate visible range for virtualized lists
fn calc_visible_range(scroll: f32, viewport: f32, total: usize, row_h: f32) -> (usize, usize, f32, f32) {
    let vp = if viewport > 0.0 { viewport } else { virt::DEFAULT_VIEWPORT_HEIGHT };
    let start = ((scroll / row_h).floor() as usize).saturating_sub(virt::SCROLL_BUFFER);
    let end = (start + (vp / row_h).ceil() as usize + 2 * virt::SCROLL_BUFFER).min(total);
    (start, end, start as f32 * row_h, total.saturating_sub(end) as f32 * row_h)
}

/// Renders the enrichment section
fn enrichment_section(state: &LoadedState) -> Element<'_, Message> {
    let e = &state.enrichment;
    
    // Tool status indicator
    let tool_status: Element<Message> = if e.fpcalc_available {
        row![icon_sized(icons::CHECK, 12).color([0.2, 0.6, 0.2]), text(" fpcalc ready").size(12).color([0.2, 0.6, 0.2])].into()
    } else {
        row![icon_sized(icons::X, 12).color([0.8, 0.2, 0.2]), text(" fpcalc missing").size(12).color([0.8, 0.2, 0.2])].into()
    };
    
    // API key input
    let api_key_input = text_input("AcoustID API Key", &e.api_key)
        .on_input(Message::EnrichmentApiKeyChanged)
        .padding(8)
        .width(Length::Fill);
    
    // Selected track display
    let selected_text = if let Some(idx) = e.selected_track {
        if let Some(track) = state.tracks.get(idx) {
            format!("Selected: {} - {}", track.artist_name, track.title)
        } else {
            "No track selected".to_string()
        }
    } else {
        "Click a track to select".to_string()
    };
    
    // Identify button
    let can_identify = e.selected_track.is_some() && !e.is_identifying && e.fpcalc_available && !e.api_key.is_empty();
    let identify_btn = if can_identify {
        button("Identify Track").padding(8).on_press(Message::EnrichmentIdentifyPressed)
    } else if e.is_identifying {
        button("Identifying...").padding(8)
    } else {
        button("Identify Track").padding(8)
    };
    
    // Result display
    let result_view: Element<Message> = if let Some(ref result) = e.last_result {
        let track = &result.track;
        let write_btn = button("Write Tags to File").padding(8).on_press(Message::EnrichmentWriteTagsPressed);
        column![
            text(format!("Match: {:.0}% confidence", result.score * 100.0)).size(14).color([0.2, 0.6, 0.2]),
            text(format!("Title: {}", track.title.as_deref().unwrap_or("-"))).size(12),
            text(format!("Artist: {}", track.artist.as_deref().unwrap_or("-"))).size(12),
            text(format!("Album: {}", track.album.as_deref().unwrap_or("-"))).size(12),
            if let Some(year) = track.year {
                text(format!("Year: {}", year)).size(12)
            } else {
                text("").size(12)
            },
            Space::with_height(Length::Fixed(5.0)),
            write_btn,
        ].spacing(2).into()
    } else if let Some(ref err) = e.last_error {
        text(format!("Error: {}", err)).size(12).color([0.8, 0.2, 0.2]).into()
    } else {
        text("").size(12).into()
    };
    
    column![
        text("Identify Track").size(20),
        tool_status,
        api_key_input,
        text(selected_text).size(12).color([0.5, 0.5, 0.5]),
        identify_btn,
        result_view,
    ]
    .spacing(8)
    .into()
}
