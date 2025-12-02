//! Diagnostics pane view.

use iced::widget::{button, column, row, scrollable, text, Space};
use iced::{Element, Length};

use crate::diagnostics::CheckStatus;
use crate::ui::icons::{self, icon_sized};
use crate::ui::messages::Message;
use crate::ui::state::LoadedState;

/// Diagnostics pane
pub fn diagnostics_pane(s: &LoadedState) -> Element<'_, Message> {
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
