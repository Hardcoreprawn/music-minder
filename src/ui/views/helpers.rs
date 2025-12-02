//! Helper functions shared across view components.

use iced::widget::button;

use crate::ui::messages::Message;
use crate::ui::state::virtualization as virt;

/// Helper to create a conditionally-enabled button
pub fn action_button<'a>(label: &'a str, msg: Option<Message>) -> iced::widget::Button<'a, Message> {
    match msg {
        Some(m) => button(label).padding(10).on_press(m),
        None => button(label).padding(10),
    }
}

/// Calculate visible range for virtualized lists
pub fn calc_visible_range(scroll: f32, viewport: f32, total: usize, row_h: f32) -> (usize, usize, f32, f32) {
    let vp = if viewport > 0.0 { viewport } else { virt::DEFAULT_VIEWPORT_HEIGHT };
    let start = ((scroll / row_h).floor() as usize).saturating_sub(virt::SCROLL_BUFFER);
    let end = (start + (vp / row_h).ceil() as usize + 2 * virt::SCROLL_BUFFER).min(total);
    (start, end, start as f32 * row_h, total.saturating_sub(end) as f32 * row_h)
}
