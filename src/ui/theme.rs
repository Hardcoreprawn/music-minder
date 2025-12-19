//! Design System Theme Constants
//!
//! Centralized theme definitions for consistent UI across the application.
//! All colors, spacing, and sizing should be defined here.
//!
//! # Color Philosophy
//! - Dark theme with deep grays (not pure black)
//! - Indigo primary accent for actions
//! - Winamp green reserved for visualizations only
//! - Semantic colors for status (success/warning/error)
//!
//! # Usage
//! ```rust
//! use crate::ui::theme;
//!
//! let bg = theme::color::SURFACE;
//! let padding = theme::spacing::MD;
//! ```

use iced::Color;

// =============================================================================
// COLORS
// =============================================================================

pub mod color {
    use super::*;

    // -------------------------------------------------------------------------
    // Backgrounds (darkest to lightest)
    // -------------------------------------------------------------------------

    /// Main app background - deepest gray
    /// Hex: #121215
    pub const BASE: Color = Color::from_rgb(
        0x12 as f32 / 255.0,
        0x12 as f32 / 255.0,
        0x15 as f32 / 255.0,
    );

    /// Cards, panels, raised surfaces
    /// Hex: #1a1a1f
    pub const SURFACE: Color = Color::from_rgb(
        0x1a as f32 / 255.0,
        0x1a as f32 / 255.0,
        0x1f as f32 / 255.0,
    );

    /// Elevated surfaces, modals, dropdowns
    /// Hex: #232328
    pub const SURFACE_ELEVATED: Color = Color::from_rgb(
        0x23 as f32 / 255.0,
        0x23 as f32 / 255.0,
        0x28 as f32 / 255.0,
    );

    /// Hover states, active items, subtle highlights
    /// Hex: #2a2a30
    pub const SURFACE_HOVER: Color = Color::from_rgb(
        0x2a as f32 / 255.0,
        0x2a as f32 / 255.0,
        0x30 as f32 / 255.0,
    );

    // -------------------------------------------------------------------------
    // Borders & Dividers
    // -------------------------------------------------------------------------

    /// Subtle separation, barely visible
    /// Hex: #2a2a30
    pub const BORDER_SUBTLE: Color = Color::from_rgb(
        0x2a as f32 / 255.0,
        0x2a as f32 / 255.0,
        0x30 as f32 / 255.0,
    );

    /// Standard borders
    /// Hex: #3a3a42
    pub const BORDER: Color = Color::from_rgb(
        0x3a as f32 / 255.0,
        0x3a as f32 / 255.0,
        0x42 as f32 / 255.0,
    );

    /// Emphasized borders, focus rings
    /// Hex: #4a4a52
    pub const BORDER_STRONG: Color = Color::from_rgb(
        0x4a as f32 / 255.0,
        0x4a as f32 / 255.0,
        0x52 as f32 / 255.0,
    );

    // -------------------------------------------------------------------------
    // Text
    // -------------------------------------------------------------------------

    /// Primary text - headings, important content
    /// Hex: #f4f4f5
    pub const TEXT_PRIMARY: Color = Color::from_rgb(
        0xf4 as f32 / 255.0,
        0xf4 as f32 / 255.0,
        0xf5 as f32 / 255.0,
    );

    /// Secondary text - body, descriptions
    /// Hex: #a1a1aa
    pub const TEXT_SECONDARY: Color = Color::from_rgb(
        0xa1 as f32 / 255.0,
        0xa1 as f32 / 255.0,
        0xaa as f32 / 255.0,
    );

    /// Muted text - hints, disabled, timestamps
    /// Hex: #71717a
    pub const TEXT_MUTED: Color = Color::from_rgb(
        0x71 as f32 / 255.0,
        0x71 as f32 / 255.0,
        0x7a as f32 / 255.0,
    );

    /// Inverse text - on light/colored backgrounds
    /// Hex: #121215
    pub const TEXT_INVERSE: Color = BASE;

    // -------------------------------------------------------------------------
    // Primary Accent (Indigo)
    // -------------------------------------------------------------------------

    /// Primary action color
    /// Hex: #6366f1
    pub const PRIMARY: Color = Color::from_rgb(
        0x63 as f32 / 255.0,
        0x66 as f32 / 255.0,
        0xf1 as f32 / 255.0,
    );

    /// Primary hover state
    /// Hex: #818cf8
    pub const PRIMARY_HOVER: Color = Color::from_rgb(
        0x81 as f32 / 255.0,
        0x8c as f32 / 255.0,
        0xf8 as f32 / 255.0,
    );

    /// Primary pressed/muted state
    /// Hex: #4f46e5
    pub const PRIMARY_PRESSED: Color = Color::from_rgb(
        0x4f as f32 / 255.0,
        0x46 as f32 / 255.0,
        0xe5 as f32 / 255.0,
    );

    // -------------------------------------------------------------------------
    // Semantic Status Colors
    // -------------------------------------------------------------------------

    /// Success - playing, confirmed, lossless
    /// Hex: #22c55e
    pub const SUCCESS: Color = Color::from_rgb(
        0x22 as f32 / 255.0,
        0xc5 as f32 / 255.0,
        0x5e as f32 / 255.0,
    );

    /// Warning - needs attention, medium confidence
    /// Hex: #f59e0b
    pub const WARNING: Color = Color::from_rgb(
        0xf5 as f32 / 255.0,
        0x9e as f32 / 255.0,
        0x0b as f32 / 255.0,
    );

    /// Error - failed, destructive, low confidence
    /// Hex: #ef4444
    pub const ERROR: Color = Color::from_rgb(
        0xef as f32 / 255.0,
        0x44 as f32 / 255.0,
        0x44 as f32 / 255.0,
    );

    // -------------------------------------------------------------------------
    // Winamp Accents (use sparingly!)
    // -------------------------------------------------------------------------

    /// Classic Winamp green - visualizations only
    /// Hex: #00ff00
    pub const WINAMP_GREEN: Color = Color::from_rgb(0.0, 1.0, 0.0);

    /// Winamp amber - VU meters, warm indicators
    /// Hex: #ffaa00
    pub const WINAMP_AMBER: Color = Color::from_rgb(1.0, 0.667, 0.0);

    // -------------------------------------------------------------------------
    // Helpers
    // -------------------------------------------------------------------------

    /// Create a color with alpha transparency
    pub const fn with_alpha(color: Color, alpha: f32) -> Color {
        Color {
            r: color.r,
            g: color.g,
            b: color.b,
            a: alpha,
        }
    }

    /// Slightly lighten a color for hover states
    pub fn lighten(color: Color, amount: f32) -> Color {
        Color {
            r: (color.r + amount).min(1.0),
            g: (color.g + amount).min(1.0),
            b: (color.b + amount).min(1.0),
            a: color.a,
        }
    }

    /// Slightly darken a color for pressed states
    pub fn darken(color: Color, amount: f32) -> Color {
        Color {
            r: (color.r - amount).max(0.0),
            g: (color.g - amount).max(0.0),
            b: (color.b - amount).max(0.0),
            a: color.a,
        }
    }
}

// =============================================================================
// SPACING
// =============================================================================

pub mod spacing {
    /// Tightest spacing - icon gaps, inline elements
    pub const XS: u16 = 4;

    /// Small spacing - component padding, small gaps
    pub const SM: u16 = 8;

    /// Medium spacing - between related items
    pub const MD: u16 = 12;

    /// Default section spacing - card padding, group separation
    pub const LG: u16 = 16;

    /// Major section spacing - pane padding, large sections
    pub const XL: u16 = 24;

    /// Hero spacing - top-level separation
    pub const XXL: u16 = 32;

    /// Massive spacing - page-level
    pub const XXXL: u16 = 48;
}

// =============================================================================
// LAYOUT DIMENSIONS
// =============================================================================

pub mod layout {
    /// Sidebar width when expanded
    pub const SIDEBAR_WIDTH: u16 = 200;

    /// Sidebar width when collapsed (icons only)
    pub const SIDEBAR_COLLAPSED: u16 = 60;

    /// Context panel width (right side)
    pub const CONTEXT_PANEL_WIDTH: u16 = 320;

    /// Player bar height (bottom)
    pub const PLAYER_BAR_HEIGHT: u16 = 72;

    /// Track row height in library list
    pub const TRACK_ROW_HEIGHT: u16 = 40;

    /// Minimum content width before scrolling
    pub const MIN_CONTENT_WIDTH: u16 = 600;

    /// Cover art size in Now Playing
    pub const COVER_ART_LARGE: u16 = 300;

    /// Cover art size in player bar
    pub const COVER_ART_SMALL: u16 = 48;

    /// Icon button size
    pub const ICON_BUTTON_SIZE: u16 = 32;

    /// Scrollbar gutter - right padding for scrollable content to prevent clipping
    /// Should match or slightly exceed scrollbar width
    pub const SCROLLBAR_GUTTER: u16 = 20;
}

// =============================================================================
// TYPOGRAPHY
// =============================================================================

pub mod typography {
    /// Hero text - Now Playing track title
    pub const SIZE_HERO: u16 = 32;

    /// Title - Pane headings
    pub const SIZE_TITLE: u16 = 24;

    /// Heading - Section headings
    pub const SIZE_HEADING: u16 = 18;

    /// Body - Default text
    pub const SIZE_BODY: u16 = 14;

    /// Small - Secondary info, metadata
    pub const SIZE_SMALL: u16 = 12;

    /// Tiny - Timestamps, counts
    pub const SIZE_TINY: u16 = 10;
}

// =============================================================================
// BORDER RADIUS
// =============================================================================

pub mod radius {
    /// Small radius - buttons, inputs
    pub const SM: f32 = 4.0;

    /// Medium radius - cards
    pub const MD: f32 = 8.0;

    /// Large radius - modals, large cards
    pub const LG: f32 = 12.0;

    /// Pill shape - chips, badges
    pub const PILL: f32 = 9999.0;
}

// =============================================================================
// ANIMATION TIMING (in milliseconds)
// =============================================================================

pub mod timing {
    /// Instant feedback - button press
    pub const INSTANT: u64 = 0;

    /// Fast transitions - hover states
    pub const FAST: u64 = 100;

    /// Normal transitions - panel slides
    pub const NORMAL: u64 = 200;

    /// Slow transitions - modal opens
    pub const SLOW: u64 = 300;

    /// Relaxed - loading states
    pub const RELAXED: u64 = 500;
}

// =============================================================================
// CONTAINER STYLE HELPERS
// =============================================================================

use iced::Border;
use iced::widget::container;

/// Create a standard container style with the given background color
pub fn container_style(bg: Color) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(bg)),
        border: Border::default(),
        ..Default::default()
    }
}

/// Container with background and border
pub fn container_bordered(bg: Color, border_color: Color) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(bg)),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: radius::MD.into(),
        },
        ..Default::default()
    }
}

/// Card style - surface background with subtle border and radius
pub fn card_style() -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(color::SURFACE)),
        border: Border {
            color: color::BORDER_SUBTLE,
            width: 1.0,
            radius: radius::MD.into(),
        },
        ..Default::default()
    }
}

/// Elevated card style - for modals and overlays
pub fn card_elevated_style() -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(color::SURFACE_ELEVATED)),
        border: Border {
            color: color::BORDER,
            width: 1.0,
            radius: radius::LG.into(),
        },
        ..Default::default()
    }
}

// =============================================================================
// BUTTON STYLE HELPERS
// =============================================================================

use iced::Theme;
use iced::widget::button;

/// Primary button - filled with accent color
pub fn button_primary(_theme: &Theme, status: button::Status) -> button::Style {
    let (bg, text) = match status {
        button::Status::Active => (color::PRIMARY, color::TEXT_PRIMARY),
        button::Status::Hovered => (color::PRIMARY_HOVER, color::TEXT_PRIMARY),
        button::Status::Pressed => (color::PRIMARY_PRESSED, color::TEXT_PRIMARY),
        button::Status::Disabled => (color::SURFACE_HOVER, color::TEXT_MUTED),
    };

    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: text,
        border: Border {
            radius: radius::SM.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Secondary button - outlined
pub fn button_secondary(_theme: &Theme, status: button::Status) -> button::Style {
    let (bg, border, text) = match status {
        button::Status::Active => (color::SURFACE, color::BORDER, color::TEXT_SECONDARY),
        button::Status::Hovered => (color::SURFACE_HOVER, color::BORDER, color::TEXT_PRIMARY),
        button::Status::Pressed => (
            color::SURFACE_ELEVATED,
            color::BORDER_STRONG,
            color::TEXT_PRIMARY,
        ),
        button::Status::Disabled => (color::SURFACE, color::BORDER_SUBTLE, color::TEXT_MUTED),
    };

    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: text,
        border: Border {
            color: border,
            width: 1.0,
            radius: radius::SM.into(),
        },
        ..Default::default()
    }
}

/// Ghost button - minimal, for less important actions
pub fn button_ghost(_theme: &Theme, status: button::Status) -> button::Style {
    let (bg, text) = match status {
        button::Status::Active => (Color::TRANSPARENT, color::TEXT_MUTED),
        button::Status::Hovered => (color::SURFACE_HOVER, color::TEXT_SECONDARY),
        button::Status::Pressed => (color::SURFACE_ELEVATED, color::TEXT_PRIMARY),
        button::Status::Disabled => (Color::TRANSPARENT, color::TEXT_MUTED),
    };

    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: text,
        border: Border {
            radius: radius::SM.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Danger button - for destructive actions
pub fn button_danger(_theme: &Theme, status: button::Status) -> button::Style {
    let (bg, text) = match status {
        button::Status::Active => (color::ERROR, color::TEXT_PRIMARY),
        button::Status::Hovered => (color::lighten(color::ERROR, 0.1), color::TEXT_PRIMARY),
        button::Status::Pressed => (color::darken(color::ERROR, 0.1), color::TEXT_PRIMARY),
        button::Status::Disabled => (color::SURFACE_HOVER, color::TEXT_MUTED),
    };

    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: text,
        border: Border {
            radius: radius::SM.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Icon button - compact, for toolbar actions
pub fn button_icon(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Active => Color::TRANSPARENT,
        button::Status::Hovered => color::SURFACE_HOVER,
        button::Status::Pressed => color::SURFACE_ELEVATED,
        button::Status::Disabled => Color::TRANSPARENT,
    };

    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: color::TEXT_SECONDARY,
        border: Border {
            radius: radius::SM.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Active/selected state button (e.g., active nav item, toggled filter)
pub fn button_active(_theme: &Theme, status: button::Status) -> button::Style {
    let (bg, text) = match status {
        button::Status::Active => (color::PRIMARY, color::TEXT_PRIMARY),
        button::Status::Hovered => (color::PRIMARY_HOVER, color::TEXT_PRIMARY),
        button::Status::Pressed => (color::PRIMARY_PRESSED, color::TEXT_PRIMARY),
        button::Status::Disabled => (color::SURFACE_HOVER, color::TEXT_MUTED),
    };

    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: text,
        border: Border {
            radius: radius::SM.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Navigation button - inactive state with hover effect
pub fn button_nav(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Active => Color::TRANSPARENT,
        button::Status::Hovered => color::SURFACE_HOVER,
        button::Status::Pressed => color::SURFACE_ELEVATED,
        button::Status::Disabled => Color::TRANSPARENT,
    };

    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: color::TEXT_SECONDARY,
        border: Border {
            radius: radius::SM.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Navigation button - active/selected state
pub fn button_nav_active(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Active => color::PRIMARY,
        button::Status::Hovered => color::PRIMARY_HOVER,
        button::Status::Pressed => color::PRIMARY_PRESSED,
        button::Status::Disabled => color::SURFACE_HOVER,
    };

    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: color::TEXT_PRIMARY,
        border: Border {
            radius: radius::SM.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

// =============================================================================
// TEXT INPUT STYLE HELPERS
// =============================================================================

use iced::widget::text_input;

/// Standard text input style
pub fn text_input_style(_theme: &Theme, status: text_input::Status) -> text_input::Style {
    let (bg, border, placeholder, value, selection) = match status {
        text_input::Status::Active => (
            color::SURFACE,
            color::BORDER_SUBTLE,
            color::TEXT_MUTED,
            color::TEXT_PRIMARY,
            color::PRIMARY,
        ),
        text_input::Status::Hovered => (
            color::SURFACE,
            color::BORDER,
            color::TEXT_MUTED,
            color::TEXT_PRIMARY,
            color::PRIMARY,
        ),
        text_input::Status::Focused => (
            color::SURFACE,
            color::PRIMARY,
            color::TEXT_MUTED,
            color::TEXT_PRIMARY,
            color::PRIMARY,
        ),
        text_input::Status::Disabled => (
            color::SURFACE,
            color::BORDER_SUBTLE,
            color::TEXT_MUTED,
            color::TEXT_MUTED,
            color::BORDER,
        ),
    };

    text_input::Style {
        background: iced::Background::Color(bg),
        border: Border {
            color: border,
            width: 1.0,
            radius: radius::SM.into(),
        },
        icon: color::TEXT_MUTED,
        placeholder,
        value,
        selection,
    }
}

// =============================================================================
// SCROLLABLE STYLE HELPERS
// =============================================================================

use iced::widget::scrollable;
use iced::widget::scrollable::{Rail, Scroller, Status as ScrollStatus};

/// Standard scrollbar style
pub fn scrollbar_style(_theme: &Theme, status: ScrollStatus) -> scrollable::Style {
    let (rail_bg, scroller_color) = match status {
        ScrollStatus::Active => (color::SURFACE, color::BORDER),
        ScrollStatus::Hovered { .. } => (color::SURFACE_HOVER, color::BORDER_STRONG),
        ScrollStatus::Dragged { .. } => (color::SURFACE_HOVER, color::PRIMARY),
    };

    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: Rail {
            background: Some(iced::Background::Color(rail_bg)),
            border: Border::default(),
            scroller: Scroller {
                color: scroller_color,
                border: Border {
                    radius: radius::PILL.into(),
                    ..Default::default()
                },
            },
        },
        horizontal_rail: Rail {
            background: Some(iced::Background::Color(rail_bg)),
            border: Border::default(),
            scroller: Scroller {
                color: scroller_color,
                border: Border {
                    radius: radius::PILL.into(),
                    ..Default::default()
                },
            },
        },
        gap: None,
    }
}

// =============================================================================
// SLIDER STYLE HELPERS
// =============================================================================

use iced::widget::slider;
use iced::widget::slider::{Handle, HandleShape, Rail as SliderRail};

/// Standard slider style (volume, seek)
pub fn slider_style(_theme: &Theme, status: slider::Status) -> slider::Style {
    let (rail_bg, rail_fill, handle_color) = match status {
        slider::Status::Active => (color::SURFACE_HOVER, color::PRIMARY, color::TEXT_PRIMARY),
        slider::Status::Hovered => (
            color::SURFACE_HOVER,
            color::PRIMARY_HOVER,
            color::TEXT_PRIMARY,
        ),
        slider::Status::Dragged => (
            color::SURFACE_HOVER,
            color::PRIMARY_PRESSED,
            color::TEXT_PRIMARY,
        ),
    };

    slider::Style {
        rail: SliderRail {
            backgrounds: (
                iced::Background::Color(rail_fill),
                iced::Background::Color(rail_bg),
            ),
            width: 4.0,
            border: Border {
                radius: 2.0.into(),
                ..Default::default()
            },
        },
        handle: Handle {
            shape: HandleShape::Circle { radius: 6.0 },
            background: iced::Background::Color(handle_color),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        },
    }
}

// =============================================================================
// PROGRESS BAR HELPERS
// =============================================================================

use iced::widget::progress_bar;

/// Standard progress bar style
pub fn progress_bar_style(_theme: &Theme) -> progress_bar::Style {
    progress_bar::Style {
        background: iced::Background::Color(color::SURFACE_HOVER),
        bar: iced::Background::Color(color::PRIMARY),
        border: Border {
            radius: 2.0.into(),
            ..Default::default()
        },
    }
}

/// Success progress bar (e.g., completed tasks)
pub fn progress_bar_success(_theme: &Theme) -> progress_bar::Style {
    progress_bar::Style {
        background: iced::Background::Color(color::SURFACE_HOVER),
        bar: iced::Background::Color(color::SUCCESS),
        border: Border {
            radius: 2.0.into(),
            ..Default::default()
        },
    }
}

// =============================================================================
// PICK LIST STYLE HELPERS
// =============================================================================

use iced::overlay::menu;
use iced::widget::pick_list;

/// Icon-only pick list style - minimal, just shows dropdown arrow
pub fn pick_list_icon_only(_theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let bg = match status {
        pick_list::Status::Active => color::SURFACE_ELEVATED,
        pick_list::Status::Hovered => color::SURFACE_HOVER,
        pick_list::Status::Opened => color::SURFACE_HOVER,
    };

    pick_list::Style {
        text_color: color::TEXT_SECONDARY,
        placeholder_color: color::TEXT_MUTED,
        handle_color: color::TEXT_SECONDARY,
        background: iced::Background::Color(bg),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 4.0.into(),
        },
    }
}

/// Menu style for pick list dropdowns
pub fn pick_list_menu(_theme: &Theme) -> menu::Style {
    menu::Style {
        text_color: color::TEXT_PRIMARY,
        background: iced::Background::Color(color::SURFACE_ELEVATED),
        border: Border {
            color: color::BORDER,
            width: 1.0,
            radius: 6.0.into(),
        },
        selected_text_color: color::TEXT_PRIMARY,
        selected_background: iced::Background::Color(color::PRIMARY),
    }
}
