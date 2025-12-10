//! UI Debug and State Tracing
//!
//! Provides structured logging for debugging UI state synchronization issues.
//!
//! # Usage
//!
//! Enable debug logging with `RUST_LOG=music_minder::ui::debug=debug`
//!
//! # Log Format
//!
//! ```text
//! [UI:state] PlayerState { status: Playing, position: 00:05, volume: 0.8 }
//! [UI:msg]   PlayerPlay received
//! [UI:sync]  sync_state() called, status: Stopped → Playing
//! [UI:smtc]  Updating SMTC playback state: Playing
//! ```

use crate::player::{PlaybackStatus, PlayerState};
use std::fmt;

/// Trait for types that can provide a debug summary.
pub trait DebugSummary {
    fn debug_summary(&self) -> String;
}

impl DebugSummary for PlayerState {
    fn debug_summary(&self) -> String {
        format!(
            "status={:?} pos={} vol={:.0}% track={}",
            self.status,
            self.position_str(),
            self.volume * 100.0,
            self.current_track
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "none".into())
        )
    }
}

impl DebugSummary for PlaybackStatus {
    fn debug_summary(&self) -> String {
        match self {
            PlaybackStatus::Playing => "▶ Playing",
            PlaybackStatus::Paused => "⏸ Paused",
            PlaybackStatus::Stopped => "⏹ Stopped",
            PlaybackStatus::Loading => "⏳ Loading",
        }
        .to_string()
    }
}

/// State transition event for debugging.
#[derive(Debug, Clone)]
pub struct StateTransition {
    pub component: &'static str,
    pub event: String,
    pub old_state: Option<String>,
    pub new_state: Option<String>,
}

impl fmt::Display for StateTransition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.old_state, &self.new_state) {
            (Some(old), Some(new)) => {
                write!(f, "[{}] {} | {} → {}", self.component, self.event, old, new)
            }
            (None, Some(new)) => {
                write!(f, "[{}] {} | → {}", self.component, self.event, new)
            }
            (Some(old), None) => {
                write!(f, "[{}] {} | {} →", self.component, self.event, old)
            }
            (None, None) => {
                write!(f, "[{}] {}", self.component, self.event)
            }
        }
    }
}

/// Log a UI message being processed.
#[macro_export]
macro_rules! ui_trace_msg {
    ($msg:expr) => {
        tracing::debug!(target: "ui::msg", "[UI:msg] {:?}", $msg);
    };
    ($msg:expr, $($arg:tt)*) => {
        tracing::debug!(target: "ui::msg", "[UI:msg] {:?} - {}", $msg, format!($($arg)*));
    };
}

/// Log a state synchronization event.
#[macro_export]
macro_rules! ui_trace_sync {
    ($old:expr, $new:expr) => {
        tracing::debug!(
            target: "ui::sync",
            "[UI:sync] state sync | {} → {}",
            $old.debug_summary(),
            $new.debug_summary()
        );
    };
    ($event:expr) => {
        tracing::debug!(target: "ui::sync", "[UI:sync] {}", $event);
    };
}

/// Log a state change in a specific component.
#[macro_export]
macro_rules! ui_trace_state {
    ($component:literal, $event:expr, $state:expr) => {
        tracing::debug!(
            target: "ui::state",
            "[UI:{}] {} | {}",
            $component,
            $event,
            $state.debug_summary()
        );
    };
    ($component:literal, $event:expr) => {
        tracing::debug!(target: "ui::state", "[UI:{}] {}", $component, $event);
    };
}

/// Log an optimistic update (UI assumes success before confirmation).
#[macro_export]
macro_rules! ui_trace_optimistic {
    ($action:expr, $expected:expr) => {
        tracing::debug!(
            target: "ui::optimistic",
            "[UI:opt] {} → expecting {}",
            $action,
            $expected
        );
    };
}

// Re-export for convenience
pub use ui_trace_msg;
pub use ui_trace_optimistic;
pub use ui_trace_state;
pub use ui_trace_sync;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::PlayerState;

    #[test]
    fn test_player_state_debug_summary() {
        let state = PlayerState::default();
        let summary = state.debug_summary();
        assert!(summary.contains("status="));
        assert!(summary.contains("vol="));
    }

    #[test]
    fn test_playback_status_debug_summary() {
        assert!(PlaybackStatus::Playing.debug_summary().contains("Playing"));
        assert!(PlaybackStatus::Paused.debug_summary().contains("Paused"));
        assert!(PlaybackStatus::Stopped.debug_summary().contains("Stopped"));
    }

    #[test]
    fn test_state_transition_display() {
        let transition = StateTransition {
            component: "player",
            event: "play pressed".to_string(),
            old_state: Some("Stopped".to_string()),
            new_state: Some("Playing".to_string()),
        };
        let display = transition.to_string();
        assert!(display.contains("[player]"));
        assert!(display.contains("Stopped"));
        assert!(display.contains("Playing"));
    }
}
