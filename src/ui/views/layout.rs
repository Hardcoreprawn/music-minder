//! Layout composition and main pane structure.

use crate::ui::icons::{self, icon_sized};
use crate::ui::messages::Message;
use crate::ui::state::{ActivePane, LoadedState};
use crate::ui::theme::{self, color, layout, spacing, typography};
use iced::widget::{Space, button, column, container, row, scrollable, text, tooltip};
use iced::{Element, Length};

use super::diagnostics_view::diagnostics_pane;
use super::enrich::enrich_pane;
use super::library::library_pane;
use super::player::player_controls;
use super::settings::settings_pane;

/// Main loaded state view - integrated layout with sidebar
pub fn loaded_view(s: &LoadedState) -> Element<'_, Message> {
    let sidebar = sidebar_view(s);
    let main_content = match s.active_pane {
        ActivePane::Library => library_pane(s),
        ActivePane::NowPlaying => now_playing_pane(s),
        ActivePane::Enrich => enrich_pane(s),
        ActivePane::Settings => settings_pane(s),
        ActivePane::Diagnostics => diagnostics_pane(s),
    };

    // Player controls always visible at bottom
    let player_bar = player_controls(s);

    // Main content area with BASE background
    let content_area = container(main_content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(spacing::XL)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(color::BASE)),
            ..Default::default()
        });

    column![row![sidebar, content_area].height(Length::Fill), player_bar,]
        .spacing(0)
        .into()
}

/// Watcher status indicator - shows if background scanning is active
fn watcher_status_indicator(s: &LoadedState, collapsed: bool) -> Element<'_, Message> {
    if collapsed {
        // Collapsed: just show a dot indicator
        let (icon, color) = if s.watcher_state.active {
            if s.watcher_state.pending_changes > 0 {
                (icons::SYNC, color::SUCCESS)
            } else {
                (icons::EYE, color::SUCCESS)
            }
        } else {
            (icons::EYE_SLASH, color::TEXT_MUTED)
        };
        container(icon_sized(icon, typography::SIZE_SMALL).color(color))
            .center_x(Length::Fill)
            .into()
    } else {
        // Expanded: full status text
        let status_text: Element<Message> = if s.watcher_state.active {
            if s.watcher_state.pending_changes > 0 {
                // Animated spinner while syncing - fixed width container prevents jank
                let spinner_char = icons::spinner_frame(s.animation_tick);
                row![
                    container(
                        text(spinner_char)
                            .size(typography::SIZE_TINY)
                            .color(color::SUCCESS)
                    )
                    .width(Length::Fixed(12.0))
                    .center_x(Length::Fixed(12.0)),
                    text(format!("Syncing {}...", s.watcher_state.pending_changes))
                        .size(typography::SIZE_TINY)
                        .color(color::SUCCESS),
                ]
                .spacing(spacing::XS)
                .into()
            } else {
                row![
                    text("●").size(8).color(color::SUCCESS),
                    text(" Watching")
                        .size(typography::SIZE_TINY)
                        .color(color::TEXT_MUTED),
                ]
                .spacing(spacing::XS)
                .into()
            }
        } else {
            text("Not watching")
                .size(typography::SIZE_TINY)
                .color(color::TEXT_MUTED)
                .into()
        };

        // Refresh button - disabled while scanning
        let refresh_btn = button(icon_sized(icons::ARROW_ROTATE, typography::SIZE_TINY))
            .padding([spacing::XS, spacing::SM])
            .style(theme::button_ghost);

        let refresh_btn = if s.is_scanning {
            refresh_btn
        } else {
            refresh_btn.on_press(Message::RescanLibrary)
        };

        row![status_text, Space::with_width(Length::Fill), refresh_btn]
            .spacing(spacing::SM)
            .align_y(iced::Alignment::Center)
            .into()
    }
}

/// Horizontal divider for sidebar sections
fn sidebar_divider() -> Element<'static, Message> {
    container(Space::new(Length::Fill, Length::Fixed(1.0)))
        .style(|_| container::Style {
            background: Some(iced::Background::Color(color::BORDER_SUBTLE)),
            ..Default::default()
        })
        .padding([0, spacing::XS])
        .into()
}

/// Sidebar with navigation and status
fn sidebar_view(s: &LoadedState) -> Element<'_, Message> {
    let collapsed = s.sidebar_collapsed;
    let sidebar_width = if collapsed {
        layout::SIDEBAR_COLLAPSED as f32
    } else {
        layout::SIDEBAR_WIDTH as f32
    };

    let is_library = s.active_pane == ActivePane::Library;
    let is_playing = s.active_pane == ActivePane::NowPlaying;
    let is_enrich = s.active_pane == ActivePane::Enrich;
    let is_settings = s.active_pane == ActivePane::Settings;
    let is_diagnostics = s.active_pane == ActivePane::Diagnostics;

    // Track count for stats section
    let track_count = s.tracks.len();

    // System status indicator - returns (icon_char, color, label, is_loading)
    let system_status: (char, iced::Color, &str, bool) = if let Some(ref diag) = s.diagnostics {
        let (status_icon, status_color, status_label) = match diag.overall_rating {
            crate::diagnostics::AudioReadiness::Excellent => {
                (icons::CIRCLE_CHECK, color::SUCCESS, "Excellent")
            }
            crate::diagnostics::AudioReadiness::Good => {
                (icons::CIRCLE_CHECK, color::SUCCESS, "Good")
            }
            crate::diagnostics::AudioReadiness::Fair => {
                (icons::CIRCLE_EXCLAIM, color::WARNING, "Fair")
            }
            crate::diagnostics::AudioReadiness::Poor => (icons::CIRCLE_XMARK, color::ERROR, "Poor"),
        };
        (status_icon, status_color, status_label, false)
    } else {
        // Use a placeholder char - we'll render the spinner text directly
        (' ', color::TEXT_MUTED, "...", true)
    };

    // Helper to render the system status icon (animated spinner when loading)
    let render_status_icon = |size: u16| -> Element<'_, Message> {
        if system_status.3 {
            // Loading: use animated ASCII spinner in fixed-width container to prevent jitter
            let spinner_char = icons::spinner_frame(s.animation_tick);
            container(text(spinner_char).size(size).color(system_status.1))
                .width(Length::Fixed(size as f32))
                .center_x(Length::Fixed(size as f32))
                .into()
        } else {
            // Loaded: use Font Awesome icon
            icon_sized(system_status.0, size)
                .color(system_status.1)
                .into()
        }
    };

    // Nav button helper - creates consistent styled buttons with proper icon/text alignment
    let nav_button = |icon: char,
                      label: &'static str,
                      is_active: bool,
                      pane: ActivePane|
     -> Element<'_, Message> {
        let (icon_color, text_color) = if is_active {
            (color::TEXT_PRIMARY, color::TEXT_PRIMARY)
        } else {
            (color::TEXT_MUTED, color::TEXT_SECONDARY)
        };

        let style_fn = if is_active {
            theme::button_nav_active
        } else {
            theme::button_nav
        };

        if collapsed {
            // Collapsed: icon only with tooltip
            let btn = button(
                container(icon_sized(icon, typography::SIZE_BODY).color(icon_color))
                    .center_x(Length::Fill)
                    .center_y(Length::Fill),
            )
            .padding(spacing::SM)
            .width(Length::Fill)
            .height(Length::Fixed(40.0))
            .style(style_fn)
            .on_press(Message::SwitchPane(pane));

            tooltip(btn, label, tooltip::Position::Right)
                .gap(spacing::SM as f32)
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(color::SURFACE_ELEVATED)),
                    border: iced::Border {
                        color: color::BORDER,
                        width: 1.0,
                        radius: 4.0.into(),
                    },
                    ..Default::default()
                })
                .into()
        } else {
            // Expanded: icon + label
            button(
                row![
                    container(icon_sized(icon, typography::SIZE_BODY).color(icon_color))
                        .width(Length::Fixed(24.0)),
                    text(label).size(typography::SIZE_BODY).color(text_color),
                ]
                .spacing(spacing::SM)
                .align_y(iced::Alignment::Center),
            )
            .padding([spacing::SM, spacing::MD])
            .width(Length::Fill)
            .style(style_fn)
            .on_press(Message::SwitchPane(pane))
            .into()
        }
    };

    // Toggle collapse button
    let toggle_icon = if collapsed {
        icons::CHEVRON_RIGHT
    } else {
        icons::CHEVRON_LEFT
    };
    let toggle_btn = button(
        container(icon_sized(toggle_icon, typography::SIZE_SMALL).color(color::TEXT_MUTED))
            .center_x(Length::Fill),
    )
    .padding([spacing::XS, spacing::SM])
    .width(Length::Fill)
    .style(theme::button_ghost)
    .on_press(Message::ToggleSidebar);

    // Build sidebar content based on collapsed state
    let sidebar_content: Element<Message> = if collapsed {
        // Collapsed sidebar: icons only
        column![
            // App icon (music note as logo)
            container(icon_sized(icons::MUSIC, typography::SIZE_TITLE).color(color::PRIMARY))
                .padding([spacing::SM, 0])
                .center_x(Length::Fill),
            Space::with_height(spacing::SM),
            sidebar_divider(),
            Space::with_height(spacing::SM),
            // Navigation icons
            nav_button(
                icons::MUSIC,
                "Now Playing",
                is_playing,
                ActivePane::NowPlaying
            ),
            nav_button(icons::LIST, "Library", is_library, ActivePane::Library),
            nav_button(icons::WAND, "Enrich", is_enrich, ActivePane::Enrich),
            nav_button(icons::GEAR, "Settings", is_settings, ActivePane::Settings),
            Space::with_height(Length::Fill),
            // Status section (compact)
            sidebar_divider(),
            Space::with_height(spacing::SM),
            watcher_status_indicator(s, true),
            Space::with_height(spacing::XS),
            // Track count as icon with tooltip
            tooltip(
                container(icon_sized(icons::DISC, typography::SIZE_SMALL).color(color::TEXT_MUTED))
                    .center_x(Length::Fill),
                text(format!("{} tracks", track_count)).size(typography::SIZE_SMALL),
                tooltip::Position::Right
            )
            .gap(spacing::SM as f32)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(color::SURFACE_ELEVATED)),
                border: iced::Border {
                    color: color::BORDER,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            }),
            Space::with_height(spacing::SM),
            sidebar_divider(),
            Space::with_height(spacing::SM),
            // System status (icon only with tooltip)
            tooltip(
                button(
                    container(render_status_icon(typography::SIZE_SMALL)).center_x(Length::Fill)
                )
                .padding(spacing::SM)
                .width(Length::Fill)
                .style(if is_diagnostics {
                    theme::button_nav_active
                } else {
                    theme::button_nav
                })
                .on_press(Message::SwitchPane(ActivePane::Diagnostics)),
                text(format!("System: {}", system_status.2)).size(typography::SIZE_SMALL),
                tooltip::Position::Right
            )
            .gap(spacing::SM as f32)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(color::SURFACE_ELEVATED)),
                border: iced::Border {
                    color: color::BORDER,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            }),
            Space::with_height(spacing::SM),
            sidebar_divider(),
            Space::with_height(spacing::SM),
            // Toggle expand button
            toggle_btn,
        ]
        .spacing(spacing::XS)
        .width(Length::Fixed(sidebar_width))
        .into()
    } else {
        // Expanded sidebar: full content
        column![
            // App title / logo area
            container(
                text("Music Minder")
                    .size(typography::SIZE_TITLE)
                    .color(color::TEXT_PRIMARY)
            )
            .padding([spacing::SM, 0]),
            Space::with_height(spacing::MD),
            sidebar_divider(),
            Space::with_height(spacing::MD),
            // Navigation buttons
            nav_button(
                icons::MUSIC,
                "Now Playing",
                is_playing,
                ActivePane::NowPlaying
            ),
            nav_button(icons::LIST, "Library", is_library, ActivePane::Library),
            nav_button(icons::WAND, "Enrich", is_enrich, ActivePane::Enrich),
            nav_button(icons::GEAR, "Settings", is_settings, ActivePane::Settings),
            Space::with_height(Length::Fill),
            // Stats section header
            sidebar_divider(),
            Space::with_height(spacing::MD),
            text("Status")
                .size(typography::SIZE_TINY)
                .color(color::TEXT_MUTED),
            Space::with_height(spacing::SM),
            // Watcher status with icon
            watcher_status_indicator(s, false),
            // Track count
            row![
                icon_sized(icons::DISC, typography::SIZE_SMALL).color(color::TEXT_MUTED),
                Space::with_width(spacing::XS),
                text(format!("{} tracks", track_count))
                    .size(typography::SIZE_SMALL)
                    .color(color::TEXT_MUTED),
            ]
            .align_y(iced::Alignment::Center),
            Space::with_height(spacing::MD),
            sidebar_divider(),
            Space::with_height(spacing::MD),
            // System status section - clickable to go to Diagnostics
            button(
                row![
                    text("System")
                        .size(typography::SIZE_SMALL)
                        .color(color::TEXT_MUTED),
                    Space::with_width(Length::Fill),
                    row![
                        render_status_icon(typography::SIZE_SMALL),
                        Space::with_width(spacing::XS),
                        text(system_status.2)
                            .size(typography::SIZE_SMALL)
                            .color(system_status.1),
                    ]
                    .align_y(iced::Alignment::Center),
                ]
                .align_y(iced::Alignment::Center)
                .width(Length::Fill)
            )
            .padding([spacing::SM, spacing::XS])
            .width(Length::Fill)
            .style(if is_diagnostics {
                theme::button_nav_active
            } else {
                theme::button_nav
            })
            .on_press(Message::SwitchPane(ActivePane::Diagnostics)),
            Space::with_height(spacing::SM),
            sidebar_divider(),
            Space::with_height(spacing::SM),
            // Toggle collapse button
            toggle_btn,
        ]
        .spacing(spacing::XS)
        .width(Length::Fixed(sidebar_width))
        .into()
    };

    container(sidebar_content)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(color::SURFACE)),
            border: iced::Border {
                color: color::BORDER_SUBTLE,
                width: 1.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        })
        .padding(spacing::MD)
        .height(Length::Fill)
        .width(Length::Fixed(sidebar_width))
        .into()
}

/// Now Playing pane - cover art, track info, and queue
fn now_playing_pane(s: &LoadedState) -> Element<'_, Message> {
    use iced::widget::image;

    let state = &s.player_state;

    // Current track info - use metadata if available
    let (track_name, artist_name, album_name) = if let Some(track) = s.current_track_info() {
        (
            track.title.clone(),
            track.artist_name.clone(),
            track.album_name.clone(),
        )
    } else if let Some(ref path) = state.current_track {
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        (name, String::new(), String::new())
    } else {
        ("No Track Playing".to_string(), String::new(), String::new())
    };

    // Cover art display (300x300 per design doc)
    let cover_size = layout::COVER_ART_LARGE as f32;
    let cover_widget: Element<Message> = if let Some(ref cover) = s.cover_art.current {
        container(
            image(image::Handle::from_bytes(cover.data.clone()))
                .width(Length::Fixed(cover_size))
                .height(Length::Fixed(cover_size)),
        )
        .style(|_| container::Style {
            border: iced::Border {
                color: color::BORDER_SUBTLE,
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        })
        .into()
    } else if s.cover_art.loading {
        container(icon_sized(icons::SPINNER, typography::SIZE_TITLE).color(color::TEXT_MUTED))
            .width(Length::Fixed(cover_size))
            .height(Length::Fixed(cover_size))
            .center_x(Length::Fixed(cover_size))
            .center_y(Length::Fixed(cover_size))
            .style(|_| container::Style {
                background: Some(iced::Background::Color(color::SURFACE_ELEVATED)),
                border: iced::Border {
                    color: color::BORDER_SUBTLE,
                    width: 1.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            })
            .into()
    } else {
        container(
            column![
                icon_sized(icons::MUSIC, 64).color(color::TEXT_MUTED),
                text("No Cover")
                    .size(typography::SIZE_SMALL)
                    .color(color::TEXT_MUTED),
            ]
            .align_x(iced::Alignment::Center)
            .spacing(spacing::SM),
        )
        .width(Length::Fixed(cover_size))
        .height(Length::Fixed(cover_size))
        .center_x(Length::Fixed(cover_size))
        .center_y(Length::Fixed(cover_size))
        .style(|_| container::Style {
            background: Some(iced::Background::Color(color::SURFACE_ELEVATED)),
            border: iced::Border {
                color: color::BORDER_SUBTLE,
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        })
        .into()
    };

    // Track info section (right of cover art)
    let track_info_section = if state.current_track.is_some() {
        // Format info line (e.g., "FLAC • 44.1kHz • 16bit")
        let format_info = state.format_info();

        // Cover source indicator
        let cover_source = if let Some(ref cover) = s.cover_art.current {
            match cover.source {
                crate::cover::CoverSource::Embedded => "Embedded",
                crate::cover::CoverSource::Sidecar(_) => "Folder art",
                crate::cover::CoverSource::Cached(_) => "Cached",
                crate::cover::CoverSource::Remote => "Remote",
            }
        } else {
            ""
        };

        column![
            // Track title (hero size)
            text(track_name)
                .size(typography::SIZE_HERO)
                .color(color::TEXT_PRIMARY),
            Space::with_height(spacing::XS),
            // Artist
            text(artist_name)
                .size(typography::SIZE_TITLE)
                .color(color::TEXT_SECONDARY),
            // Album
            text(album_name)
                .size(typography::SIZE_HEADING)
                .color(color::TEXT_MUTED),
            Space::with_height(spacing::LG),
            // Format info with lossless badge
            row![
                text(format_info)
                    .size(typography::SIZE_SMALL)
                    .color(color::TEXT_MUTED),
            ]
            .spacing(spacing::SM),
            // Cover source
            text(cover_source)
                .size(typography::SIZE_TINY)
                .color(color::TEXT_MUTED),
        ]
        .spacing(spacing::XS)
    } else {
        column![
            text("No Track Playing")
                .size(typography::SIZE_HERO)
                .color(color::TEXT_MUTED),
            Space::with_height(spacing::SM),
            text("Select a track from the library")
                .size(typography::SIZE_BODY)
                .color(color::TEXT_MUTED),
        ]
        .spacing(spacing::XS)
    };

    // Cover + Info row
    let track_display = row![
        cover_widget,
        Space::with_width(spacing::XL),
        track_info_section,
    ]
    .align_y(iced::Alignment::Start);

    // Queue display with controls
    let queue_section = {
        let (queue_len, current_idx, shuffle_on, repeat_mode) = s
            .player
            .as_ref()
            .map(|p| {
                (
                    p.queue().items().len(),
                    p.queue().current_index(),
                    p.queue().shuffle(),
                    p.queue().repeat(),
                )
            })
            .unwrap_or((0, None, false, crate::player::RepeatMode::Off));

        // Track position indicator (e.g., "Track 3 of 25")
        let position_text = if let Some(idx) = current_idx {
            text(format!("Track {} of {}", idx + 1, queue_len))
                .size(typography::SIZE_SMALL)
                .color(color::TEXT_SECONDARY)
        } else {
            text(format!("{} tracks", queue_len))
                .size(typography::SIZE_SMALL)
                .color(color::TEXT_MUTED)
        };

        // Shuffle button with active state
        let shuffle_style = if shuffle_on {
            theme::button_active
        } else {
            theme::button_ghost
        };
        let shuffle_btn = button(icon_sized(icons::SHUFFLE, typography::SIZE_SMALL))
            .padding([spacing::XS, spacing::SM])
            .style(shuffle_style)
            .on_press(Message::QueueToggleShuffle);

        // Repeat button with mode indicator
        let repeat_icon = match repeat_mode {
            crate::player::RepeatMode::One => icons::REPEAT_ONE,
            _ => icons::REPEAT,
        };
        let repeat_active = repeat_mode != crate::player::RepeatMode::Off;
        let repeat_style = if repeat_active {
            theme::button_active
        } else {
            theme::button_ghost
        };
        let repeat_btn = button(icon_sized(repeat_icon, typography::SIZE_SMALL))
            .padding([spacing::XS, spacing::SM])
            .style(repeat_style)
            .on_press(Message::QueueCycleRepeat);

        // Clear button
        let clear_btn = button(icon_sized(icons::XMARK, typography::SIZE_SMALL))
            .padding([spacing::XS, spacing::SM])
            .style(theme::button_ghost)
            .on_press(Message::QueueClear);

        let queue_header = row![
            text("Queue")
                .size(typography::SIZE_HEADING)
                .color(color::TEXT_PRIMARY),
            Space::with_width(spacing::MD),
            shuffle_btn,
            repeat_btn,
            Space::with_width(Length::Fill),
            position_text,
            Space::with_width(spacing::SM),
            clear_btn,
        ]
        .align_y(iced::Alignment::Center);

        let queue_selection = s.queue_selection;
        let queue_list = if let Some(ref player) = s.player {
            let items: Vec<Element<Message>> = player
                .queue()
                .items()
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    let is_current = player.queue().current_index() == Some(i);
                    let is_selected = queue_selection == Some(i);

                    // Priority: keyboard selection > current playing > alternating
                    let bg = if is_selected {
                        color::PRIMARY // Bright for keyboard focus
                    } else if is_current {
                        color::PRIMARY_PRESSED
                    } else if i % 2 == 0 {
                        color::SURFACE
                    } else {
                        color::BASE
                    };
                    let fg = if is_selected || is_current {
                        color::TEXT_PRIMARY
                    } else {
                        color::TEXT_SECONDARY
                    };

                    let display_text = if let Some(track) = s.track_info_by_path(&item.path) {
                        format!("{} - {}", track.artist_name, track.title)
                    } else {
                        item.path
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| "Unknown".to_string())
                    };

                    // Index number with current/selection indicator
                    let index_widget: Element<Message> = if is_current {
                        icon_sized(icons::PLAY, typography::SIZE_TINY)
                            .color(color::PRIMARY)
                            .into()
                    } else if is_selected {
                        icon_sized(icons::CHEVRON_RIGHT, typography::SIZE_TINY)
                            .color(color::PRIMARY)
                            .into()
                    } else {
                        text(format!("{}", i + 1))
                            .size(typography::SIZE_TINY)
                            .color(color::TEXT_MUTED)
                            .into()
                    };

                    // Remove button for this item
                    let remove_btn = button(icon_sized(icons::XMARK, typography::SIZE_TINY))
                        .padding([spacing::XS, spacing::SM])
                        .style(theme::button_ghost)
                        .on_press(Message::QueueRemove(i));

                    // Make the row clickable to select and set keyboard focus
                    let track_btn = button(
                        row![
                            container(index_widget).width(Length::Fixed(24.0)),
                            text(display_text).size(typography::SIZE_SMALL).color(fg),
                            Space::with_width(Length::Fill),
                        ]
                        .align_y(iced::Alignment::Center),
                    )
                    .width(Length::Fill)
                    .padding([spacing::XS, spacing::SM])
                    .style(move |_, status| {
                        let row_bg = match status {
                            button::Status::Hovered if !is_selected => color::SURFACE_HOVER,
                            _ => bg,
                        };
                        button::Style {
                            background: Some(iced::Background::Color(row_bg)),
                            text_color: fg,
                            ..Default::default()
                        }
                    })
                    .on_press(Message::QueueSelectIndex(i));

                    row![track_btn, remove_btn, Space::with_width(spacing::SM)]
                        .align_y(iced::Alignment::Center)
                        .into()
                })
                .collect();

            if items.is_empty() {
                column![
                    container(
                        text("Queue is empty — add tracks from the Library")
                            .size(typography::SIZE_SMALL)
                            .color(color::TEXT_MUTED)
                    )
                    .padding(spacing::XL)
                    .center_x(Length::Fill)
                ]
            } else {
                column(items).spacing(1)
            }
        } else {
            column![
                text("Player not initialized")
                    .size(typography::SIZE_SMALL)
                    .color(color::ERROR)
            ]
        };

        column![
            queue_header,
            Space::with_height(spacing::SM),
            scrollable(queue_list).height(Length::Fill),
        ]
        .height(Length::Fill)
    };

    // Simple layout: cover+info at top, queue takes remaining space
    column![
        track_display,
        Space::with_height(spacing::XL),
        queue_section,
    ]
    .spacing(0)
    .padding(spacing::LG)
    .height(Length::Fill)
    .into()
}
