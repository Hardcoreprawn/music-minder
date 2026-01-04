//! Toast notification component for non-blocking user feedback.
//!
//! Toasts are ephemeral messages that appear at the bottom of the screen
//! and auto-dismiss after a configurable duration.
//!
//! # Example
//! ```ignore
//! state.toasts.success("Tags written successfully");
//! state.toasts.error("Failed to save file");
//! ```

use crate::ui::icons::{self, icon_sized};
use crate::ui::messages::Message;
use crate::ui::theme::{color, spacing, typography};
use iced::widget::{Space, button, container, row, text};
use iced::{Element, Length, Padding};
use std::time::{Duration, Instant};

/// Duration before toasts auto-dismiss
pub const TOAST_DURATION: Duration = Duration::from_secs(4);

/// Maximum number of visible toasts at once
pub const MAX_VISIBLE_TOASTS: usize = 5;

/// Toast severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastLevel {
    Success,
    Error,
    Warning,
    Info,
}

impl ToastLevel {
    /// Icon for this toast level
    fn icon(&self) -> char {
        match self {
            ToastLevel::Success => icons::CIRCLE_CHECK,
            ToastLevel::Error => icons::CIRCLE_XMARK,
            ToastLevel::Warning => icons::CIRCLE_EXCLAIM,
            ToastLevel::Info => icons::CIRCLE_INFO,
        }
    }

    /// Accent color for this toast level
    fn color(&self) -> iced::Color {
        match self {
            ToastLevel::Success => color::SUCCESS,
            ToastLevel::Error => color::ERROR,
            ToastLevel::Warning => color::WARNING,
            ToastLevel::Info => color::PRIMARY,
        }
    }
}

/// A single toast notification
#[derive(Debug, Clone)]
pub struct Toast {
    /// Unique ID for this toast (for removal)
    id: u64,
    /// Severity level
    level: ToastLevel,
    /// Message to display
    message: String,
    /// When this toast was created
    created_at: Instant,
}

impl Toast {
    /// Create a new toast with auto-generated ID
    fn new(level: ToastLevel, message: impl Into<String>) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        Self {
            id: COUNTER.fetch_add(1, Ordering::Relaxed),
            level,
            message: message.into(),
            created_at: Instant::now(),
        }
    }

    /// Create a success toast
    fn success(message: impl Into<String>) -> Self {
        Self::new(ToastLevel::Success, message)
    }

    /// Create an error toast
    fn error(message: impl Into<String>) -> Self {
        Self::new(ToastLevel::Error, message)
    }

    /// Create a warning toast
    fn warning(message: impl Into<String>) -> Self {
        Self::new(ToastLevel::Warning, message)
    }

    /// Create an info toast
    fn info(message: impl Into<String>) -> Self {
        Self::new(ToastLevel::Info, message)
    }

    /// Check if this toast should be dismissed
    fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= TOAST_DURATION
    }
}

/// Container for managing multiple toasts
#[derive(Debug, Clone, Default)]
pub struct ToastQueue {
    toasts: Vec<Toast>,
}

impl ToastQueue {
    /// Create empty toast queue
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self { toasts: Vec::new() }
    }

    /// Add a toast to the queue
    pub fn push(&mut self, toast: Toast) {
        self.toasts.push(toast);
        // Keep only the most recent toasts
        if self.toasts.len() > MAX_VISIBLE_TOASTS * 2 {
            self.toasts.drain(0..MAX_VISIBLE_TOASTS);
        }
    }

    /// Remove a toast by ID
    pub fn remove(&mut self, id: u64) {
        self.toasts.retain(|t| t.id != id);
    }

    /// Remove all expired toasts
    pub fn remove_expired(&mut self) {
        self.toasts.retain(|t| !t.is_expired());
    }

    /// Get visible toasts (most recent, up to MAX_VISIBLE_TOASTS)
    pub fn visible(&self) -> impl Iterator<Item = &Toast> {
        let start = self.toasts.len().saturating_sub(MAX_VISIBLE_TOASTS);
        self.toasts[start..].iter().filter(|t| !t.is_expired())
    }

    /// Check if there are any visible toasts
    #[allow(dead_code)]
    pub fn has_visible(&self) -> bool {
        self.visible().next().is_some()
    }

    /// Convenience: add a success toast
    pub fn success(&mut self, message: impl Into<String>) {
        self.push(Toast::success(message));
    }

    /// Convenience: add an error toast
    pub fn error(&mut self, message: impl Into<String>) {
        self.push(Toast::error(message));
    }

    /// Convenience: add a warning toast
    pub fn warning(&mut self, message: impl Into<String>) {
        self.push(Toast::warning(message));
    }

    /// Convenience: add an info toast
    pub fn info(&mut self, message: impl Into<String>) {
        self.push(Toast::info(message));
    }
}

/// Render a single toast notification
fn toast_view(toast: &Toast) -> Element<'_, Message> {
    let accent = toast.level.color();

    let icon = icon_sized(toast.level.icon(), typography::SIZE_BODY).color(accent);

    let message_text = text(&toast.message)
        .size(typography::SIZE_BODY)
        .color(color::TEXT_PRIMARY);

    let dismiss_btn =
        button(icon_sized(icons::XMARK, typography::SIZE_SMALL).color(color::TEXT_MUTED))
            .padding([spacing::XS, spacing::SM])
            .style(crate::ui::theme::button_ghost)
            .on_press(Message::ToastDismiss(toast.id));

    let content = row![
        icon,
        Space::with_width(spacing::SM),
        message_text,
        Space::with_width(Length::Fill),
        dismiss_btn,
    ]
    .align_y(iced::Alignment::Center)
    .padding([spacing::SM, spacing::MD]);

    // Toast container with left accent border
    container(content)
        .width(Length::Fixed(400.0))
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(color::SURFACE_ELEVATED)),
            border: iced::Border {
                color: accent,
                width: 3.0,
                radius: 6.0.into(),
            },
            shadow: iced::Shadow {
                color: iced::Color::from_rgba(0.0, 0.0, 0.0, 0.3),
                offset: iced::Vector::new(0.0, 2.0),
                blur_radius: 8.0,
            },
            ..Default::default()
        })
        .into()
}

/// Render the toast container overlay
/// This should be stacked on top of the main content
pub fn toast_overlay(queue: &ToastQueue) -> Option<Element<'_, Message>> {
    let toasts: Vec<_> = queue.visible().collect();

    if toasts.is_empty() {
        return None;
    }

    let toast_elements: Vec<Element<Message>> = toasts.iter().map(|t| toast_view(t)).collect();

    // Stack toasts in a column at the bottom-right
    let toast_column = iced::widget::column(toast_elements)
        .spacing(spacing::SM)
        .align_x(iced::Alignment::End);

    // Position at bottom-right with padding
    // Extra bottom padding (100px) to clear the player bar
    let overlay = container(toast_column)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Right)
        .align_y(iced::alignment::Vertical::Bottom)
        .padding(Padding {
            top: 0.0,
            right: spacing::XL as f32,
            bottom: 100.0,
            left: 0.0,
        });

    Some(overlay.into())
}
