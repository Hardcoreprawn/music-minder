//! Diagnostics pane view.

use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Element, Length};

use crate::diagnostics::{AudioReadiness, CheckStatus};
use crate::ui::icons::{self, icon_sized, spinner_frame};
use crate::ui::messages::Message;
use crate::ui::state::LoadedState;
use crate::ui::theme::{self, color, layout, spacing, typography};

/// Minimum animation phases to show (7 checks)
#[allow(dead_code)]
const _MIN_ANIMATION_PHASES: u32 = 7;
/// Ticks per phase at 60fps (~1.5 seconds each)
const TICKS_PER_PHASE: u32 = 90;

/// Diagnostics pane
pub fn diagnostics_pane(s: &LoadedState) -> Element<'_, Message> {
    // Calculate animation progress for running view
    let elapsed = s.animation_tick.wrapping_sub(s.diagnostics_started_tick);

    // Show running view if still loading or have pending results (animation in progress)
    if s.diagnostics_loading || s.diagnostics_pending.is_some() {
        running_view(s, elapsed)
    } else if let Some(ref diag) = s.diagnostics {
        results_view(s, diag)
    } else {
        empty_view()
    }
}

/// View when no diagnostics have been run yet
fn empty_view() -> Element<'static, Message> {
    let icon =
        container(icon_sized(icons::SLIDERS, 48).color(color::TEXT_MUTED)).padding(spacing::LG);

    column![
        Space::with_height(Length::FillPortion(1)),
        container(
            column![
                icon,
                Space::with_height(spacing::MD),
                text("System Diagnostics")
                    .size(typography::SIZE_TITLE)
                    .color(color::TEXT_PRIMARY),
                Space::with_height(spacing::SM),
                text("Check your system's audio readiness")
                    .size(typography::SIZE_BODY)
                    .color(color::TEXT_MUTED),
                Space::with_height(spacing::LG),
                button(
                    row![
                        icon_sized(icons::PLAY, typography::SIZE_BODY),
                        Space::with_width(spacing::SM),
                        text("Run Diagnostics").size(typography::SIZE_BODY),
                    ]
                    .align_y(iced::Alignment::Center)
                )
                .padding([spacing::SM, spacing::LG])
                .style(theme::button_primary)
                .on_press(Message::DiagnosticsRunPressed),
            ]
            .align_x(iced::Alignment::Center)
        )
        .width(Length::Fill)
        .center_x(Length::Fill),
        Space::with_height(Length::FillPortion(2)),
    ]
    .into()
}

/// Animated view while diagnostics are running
fn running_view(s: &LoadedState, elapsed: u32) -> Element<'_, Message> {
    // Calculate current phase from elapsed ticks
    let phase = elapsed / TICKS_PER_PHASE;

    let checks = [
        ("SIMD Capabilities", icons::CHIP),
        ("Timer Resolution", icons::CLOCK),
        ("CPU Performance", icons::GAUGE),
        ("Memory Status", icons::MEMORY),
        ("Power Plan", icons::BOLT),
        ("Audio Devices", icons::HEADPHONES),
        ("Finalizing...", icons::CIRCLE_CHECK),
    ];

    let check_rows: Vec<Element<'_, Message>> = checks
        .iter()
        .enumerate()
        .map(|(i, (name, icon))| {
            let (status_element, text_color): (Element<'_, Message>, _) = if (i as u32) < phase {
                // Completed
                (
                    icon_sized(icons::CIRCLE_CHECK, typography::SIZE_BODY)
                        .color(color::SUCCESS)
                        .into(),
                    color::TEXT_SECONDARY,
                )
            } else if (i as u32) == phase {
                // Currently running - animated spinner
                let spinner = spinner_frame(s.animation_tick);
                (
                    container(
                        text(spinner)
                            .size(typography::SIZE_BODY)
                            .color(color::PRIMARY),
                    )
                    .width(Length::Fixed(16.0))
                    .center_x(Length::Fixed(16.0))
                    .into(),
                    color::TEXT_PRIMARY,
                )
            } else {
                // Pending
                (
                    icon_sized(icons::CIRCLE, typography::SIZE_BODY)
                        .color(color::TEXT_MUTED)
                        .into(),
                    color::TEXT_MUTED,
                )
            };

            row![
                status_element,
                Space::with_width(spacing::SM),
                icon_sized(*icon, typography::SIZE_BODY).color(text_color),
                Space::with_width(spacing::SM),
                text(*name).size(typography::SIZE_BODY).color(text_color),
            ]
            .align_y(iced::Alignment::Center)
            .into()
        })
        .collect();

    column![
        text("System Diagnostics")
            .size(typography::SIZE_TITLE)
            .color(color::TEXT_PRIMARY),
        Space::with_height(spacing::SM),
        text("Analyzing your system...")
            .size(typography::SIZE_BODY)
            .color(color::TEXT_MUTED),
        Space::with_height(spacing::XL),
        container(column(check_rows).spacing(spacing::MD))
            .padding(spacing::LG)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(color::SURFACE)),
                border: iced::Border {
                    color: color::BORDER_SUBTLE,
                    width: 1.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            }),
        Space::with_height(Length::Fill),
    ]
    .spacing(spacing::XS)
    .into()
}

/// View showing diagnostic results
fn results_view<'a>(
    s: &'a LoadedState,
    diag: &'a crate::diagnostics::DiagnosticReport,
) -> Element<'a, Message> {
    // Overall rating card
    let (rating_icon, rating_color, rating_bg) = match diag.overall_rating {
        AudioReadiness::Excellent => (icons::CIRCLE_CHECK, color::SUCCESS, [0.1, 0.3, 0.1]),
        AudioReadiness::Good => (icons::CIRCLE_CHECK, color::SUCCESS, [0.1, 0.3, 0.1]),
        AudioReadiness::Fair => (icons::CIRCLE_EXCLAIM, color::WARNING, [0.3, 0.25, 0.1]),
        AudioReadiness::Poor => (icons::CIRCLE_XMARK, color::ERROR, [0.3, 0.1, 0.1]),
    };

    let rating_card = container(
        row![
            icon_sized(rating_icon, 32).color(rating_color),
            Space::with_width(spacing::MD),
            column![
                text(format!("Audio Readiness: {}", diag.overall_rating.as_str()))
                    .size(typography::SIZE_HEADING)
                    .color(color::TEXT_PRIMARY),
                text(diag.overall_rating.description())
                    .size(typography::SIZE_SMALL)
                    .color(color::TEXT_SECONDARY),
            ]
            .spacing(spacing::XS),
        ]
        .align_y(iced::Alignment::Center),
    )
    .padding(spacing::LG)
    .width(Length::Fill)
    .style(move |_| container::Style {
        background: Some(iced::Background::Color(rating_bg.into())),
        border: iced::Border {
            color: rating_color,
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    });

    // Group checks by category - use BTreeMap for stable iteration order
    let mut categories: std::collections::BTreeMap<
        &str,
        Vec<&crate::diagnostics::DiagnosticCheck>,
    > = std::collections::BTreeMap::new();
    for check in &diag.checks {
        categories.entry(&check.category).or_default().push(check);
    }

    // Build check sections (BTreeMap gives consistent alphabetical order)
    let mut sections: Vec<Element<'a, Message>> = Vec::new();

    for (category, checks) in &categories {
        let check_rows: Vec<Element<'a, Message>> = checks
            .iter()
            .map(|check| {
                let is_expanded = s.diagnostics_expanded.contains(&check.name);
                check_row(check, is_expanded)
            })
            .collect();

        let section = column![
            text(*category)
                .size(typography::SIZE_BODY)
                .color(color::TEXT_MUTED),
            Space::with_height(spacing::SM),
            column(check_rows).spacing(spacing::SM),
        ]
        .spacing(spacing::XS);

        sections.push(section.into());
        sections.push(Space::with_height(spacing::LG).into());
    }

    // Re-run button - uses primary style to match initial button
    let rerun_button = button(
        row![
            icon_sized(icons::ARROW_ROTATE, typography::SIZE_BODY),
            Space::with_width(spacing::SM),
            text("Run Again").size(typography::SIZE_BODY),
        ]
        .align_y(iced::Alignment::Center),
    )
    .padding([spacing::SM, spacing::MD])
    .style(theme::button_primary)
    .on_press(Message::DiagnosticsRunPressed);

    let timestamp = diag.timestamp.format("%H:%M:%S").to_string();

    column![
        // Header
        row![
            text("System Diagnostics")
                .size(typography::SIZE_TITLE)
                .color(color::TEXT_PRIMARY),
            Space::with_width(Length::Fill),
            text(format!("Last run: {}", timestamp))
                .size(typography::SIZE_SMALL)
                .color(color::TEXT_MUTED),
            Space::with_width(spacing::MD),
            rerun_button,
        ]
        .align_y(iced::Alignment::Center),
        Space::with_height(spacing::LG),
        // Scrollable content with right padding for scrollbar
        scrollable(
            container(column![
                rating_card,
                Space::with_height(spacing::XL),
                column(sections),
            ])
            .padding(iced::Padding {
                top: 0.0,
                right: layout::SCROLLBAR_GUTTER as f32,
                bottom: 0.0,
                left: 0.0
            })
        )
        .height(Length::Fill),
    ]
    .spacing(spacing::XS)
    .into()
}

/// Get detailed explanation for a diagnostic check
fn get_check_explanation(name: &str, status: CheckStatus) -> (&'static str, &'static str) {
    // Returns (what_it_means, how_to_fix) based on check name
    match name {
        "Build Mode" => (
            "Rust programs can be compiled in debug mode (fast compilation, slow execution) or release mode (slow compilation, fast execution). Debug builds include extra checks and disable optimizations.",
            match status {
                CheckStatus::Pass => {
                    "You're running an optimized release build. Benchmarks and performance are accurate."
                }
                CheckStatus::Warning => {
                    "You're running a debug build. SIMD benchmarks will show poor results because optimizations are disabled. Run with 'cargo run --release' for accurate measurements."
                }
                _ => "Build mode information.",
            },
        ),
        "SIMD Acceleration" => (
            "SIMD (Single Instruction Multiple Data) allows your CPU to process multiple audio samples simultaneously, dramatically speeding up volume scaling and format conversion.",
            match status {
                CheckStatus::Pass => {
                    "Your CPU supports modern SIMD instructions (AVX2/SSE4.1). Audio processing is hardware-accelerated."
                }
                CheckStatus::Warning => {
                    "Your CPU only supports basic instructions. Audio will work but use more CPU."
                }
                _ => "SIMD detection failed. Audio processing will use fallback code.",
            },
        ),
        "SIMD Volume Scaling" => (
            "Measures how fast volume changes can be applied to audio samples. Higher speedup means less CPU usage during playback. Note: This benchmark is only accurate in release builds (`cargo run --release`).",
            match status {
                CheckStatus::Pass => {
                    "SIMD is providing good speedup. Volume processing is well-optimized."
                }
                CheckStatus::Warning | CheckStatus::Info => {
                    "SIMD shows minimal speedup. If running a debug build, this is expected — try a release build for accurate results."
                }
                _ => "Volume scaling benchmark failed.",
            },
        ),
        "SIMD f32→i16 Conversion" => (
            "Audio is processed as 32-bit floats but output as 16-bit integers. This conversion happens thousands of times per second. Note: This benchmark is only accurate in release builds (`cargo run --release`).",
            match status {
                CheckStatus::Pass => "Format conversion is hardware-accelerated with good speedup.",
                CheckStatus::Warning | CheckStatus::Info => {
                    "Conversion shows minimal speedup. If running a debug build, this is expected — SIMD optimizations require release mode."
                }
                _ => "Conversion benchmark failed.",
            },
        ),
        "Timer Resolution" => (
            "Windows timer resolution affects how precisely the audio system can schedule buffer refills. Music Minder requests 1ms resolution on startup, which is the standard for audio applications. The 'best' value shown is your hardware's theoretical minimum, but 1ms is more than sufficient for glitch-free playback.",
            match status {
                CheckStatus::Pass => {
                    "Timer is at 1ms or better — exactly what we need. Audio scheduling will be precise and reliable."
                }
                CheckStatus::Warning => {
                    "Timer resolution is slightly high. Music Minder requested 1ms but another process may be interfering."
                }
                CheckStatus::Fail => {
                    "Timer resolution is too high. This can cause audio glitches. Try closing resource-heavy applications."
                }
                _ => "Timer information is for reference.",
            },
        ),
        "Power Plan" => (
            "Windows power plans affect CPU performance. 'Balanced' or 'Power saver' modes may throttle CPU during playback.",
            match status {
                CheckStatus::Pass => {
                    "High Performance mode ensures consistent CPU speed for glitch-free audio."
                }
                CheckStatus::Warning => {
                    "Balanced mode may cause brief CPU throttling. Consider switching to High Performance for critical listening."
                }
                CheckStatus::Fail => {
                    "Power Saver mode will likely cause audio glitches. Switch to High Performance in Windows Settings > Power."
                }
                _ => "Power plan information.",
            },
        ),
        "Total RAM" | "Available RAM" => (
            "Available memory affects how much audio can be buffered and how many tracks can be loaded.",
            match status {
                CheckStatus::Pass => {
                    "Plenty of memory available for audio playback and library management."
                }
                CheckStatus::Warning => {
                    "Memory is getting low. Close unused applications if you experience issues."
                }
                CheckStatus::Fail => {
                    "Very low memory. Audio glitches likely. Close other applications."
                }
                _ => "Memory information for reference.",
            },
        ),
        "Memory Pressure" => (
            "Memory pressure indicates how much of your RAM is in active use. High pressure can cause system-wide slowdowns.",
            match status {
                CheckStatus::Pass => "Memory pressure is low. System has plenty of headroom.",
                CheckStatus::Warning => {
                    "Moderate memory pressure. Should be fine but monitor if issues occur."
                }
                CheckStatus::Fail => {
                    "High memory pressure. System may be swapping to disk, causing audio dropouts."
                }
                _ => "Memory pressure information.",
            },
        ),
        "Processor" => (
            "Your CPU model and core count. More cores help with background tasks while playing audio.",
            "This is informational. Modern multi-core processors handle audio playback easily.",
        ),
        "CPU Frequency" => (
            "Current CPU clock speed. Higher frequencies mean faster audio processing.",
            match status {
                CheckStatus::Pass => "CPU is running at a good speed for audio work.",
                CheckStatus::Warning => "CPU may be throttled. Check power settings and cooling.",
                _ => "CPU frequency information.",
            },
        ),
        "CPU Usage" => (
            "Current CPU utilization. Very high usage can cause audio buffer underruns.",
            match status {
                CheckStatus::Pass => "CPU has plenty of headroom for audio processing.",
                CheckStatus::Warning => {
                    "CPU is moderately busy. Audio should be fine but monitor for issues."
                }
                CheckStatus::Fail => "CPU is heavily loaded. Consider closing other applications.",
                _ => "CPU usage information.",
            },
        ),
        "CPU Cores" => (
            "Number of logical CPU cores available for parallel processing.",
            "More cores help run background tasks without affecting audio playback.",
        ),
        "Audio Devices" => (
            "Number of audio output devices detected on your system.",
            "Multiple devices give you flexibility in choosing where to output audio.",
        ),
        _ => (
            "This diagnostic check provides information about your system's audio capabilities.",
            "See the value for current status.",
        ),
    }
}

/// Single check row with status, value, and expandable details
fn check_row(
    check: &crate::diagnostics::DiagnosticCheck,
    is_expanded: bool,
) -> Element<'_, Message> {
    let (status_icon, status_color) = match check.status {
        CheckStatus::Pass => (icons::CIRCLE_CHECK, color::SUCCESS),
        CheckStatus::Warning => (icons::CIRCLE_EXCLAIM, color::WARNING),
        CheckStatus::Fail => (icons::CIRCLE_XMARK, color::ERROR),
        CheckStatus::Info => (icons::CIRCLE_INFO, color::PRIMARY),
    };

    // Expand/collapse chevron
    let chevron = if is_expanded {
        icons::CHEVRON_DOWN
    } else {
        icons::CHEVRON_RIGHT
    };

    let header = row![
        icon_sized(status_icon, typography::SIZE_BODY).color(status_color),
        Space::with_width(spacing::SM),
        text(&check.name)
            .size(typography::SIZE_BODY)
            .color(color::TEXT_PRIMARY),
        Space::with_width(Length::Fill),
        text(&check.value)
            .size(typography::SIZE_SMALL)
            .color(color::TEXT_SECONDARY),
        Space::with_width(spacing::SM),
        icon_sized(chevron, typography::SIZE_SMALL).color(color::TEXT_MUTED),
    ]
    .align_y(iced::Alignment::Center);

    let mut content = column![header].spacing(spacing::SM);

    // Show expanded details
    if is_expanded {
        let (explanation, status_text) = get_check_explanation(&check.name, check.status);

        // Status-specific color for the explanation box
        let (hint_bg, hint_border) = match check.status {
            CheckStatus::Pass => ([0.1, 0.2, 0.1], color::SUCCESS),
            CheckStatus::Warning => ([0.2, 0.18, 0.1], color::WARNING),
            CheckStatus::Fail => ([0.2, 0.1, 0.1], color::ERROR),
            CheckStatus::Info => ([0.1, 0.15, 0.2], color::PRIMARY),
        };

        content = content.push(
            container(
                column![
                    // What this check means
                    text(explanation)
                        .size(typography::SIZE_SMALL)
                        .color(color::TEXT_SECONDARY),
                    Space::with_height(spacing::SM),
                    // Status-specific explanation
                    row![
                        icon_sized(status_icon, typography::SIZE_SMALL).color(status_color),
                        Space::with_width(spacing::SM),
                        text(status_text)
                            .size(typography::SIZE_SMALL)
                            .color(color::TEXT_PRIMARY),
                    ]
                    .align_y(iced::Alignment::Center),
                ]
                .spacing(spacing::XS),
            )
            .padding(spacing::MD)
            .width(Length::Fill)
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(hint_bg.into())),
                border: iced::Border {
                    color: hint_border,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            }),
        );

        // Show recommendation if present (for warnings/failures)
        if let Some(ref rec) = check.recommendation
            && matches!(check.status, CheckStatus::Warning | CheckStatus::Fail)
        {
            content = content.push(
                row![
                    icon_sized(icons::LIGHTBULB, typography::SIZE_SMALL).color(color::WARNING),
                    Space::with_width(spacing::SM),
                    text(format!("Tip: {}", rec))
                        .size(typography::SIZE_SMALL)
                        .color(color::TEXT_SECONDARY),
                ]
                .align_y(iced::Alignment::Center),
            );
        }
    }

    let check_name = check.name.clone();

    button(
        container(content)
            .padding([spacing::SM, spacing::MD])
            .width(Length::Fill),
    )
    .width(Length::Fill)
    .padding(0)
    .style(|_theme, status| {
        let base = iced::widget::button::Style {
            background: Some(iced::Background::Color(color::SURFACE)),
            border: iced::Border {
                color: color::BORDER_SUBTLE,
                width: 1.0,
                radius: 6.0.into(),
            },
            text_color: color::TEXT_PRIMARY,
            ..Default::default()
        };
        match status {
            iced::widget::button::Status::Hovered => iced::widget::button::Style {
                background: Some(iced::Background::Color(color::SURFACE_ELEVATED)),
                ..base
            },
            _ => base,
        }
    })
    .on_press(Message::DiagnosticsToggleCheck(check_name))
    .into()
}
