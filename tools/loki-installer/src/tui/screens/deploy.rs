//! Deployment progress screen content.

use crate::tui::app::AppState;
use ratatui::text::{Line, Text};

pub fn content(state: &AppState) -> Text<'static> {
    let mut lines = vec![Line::from("Deployment progress"), Line::from("")];
    if let Some(phase) = state.deployment.current_phase {
        lines.push(Line::from(format!("Current phase: {phase}")));
    }
    lines.push(Line::from(""));
    lines.push(Line::from("Logs:"));
    for line in state.deployment.logs.iter().rev().take(20).rev() {
        // Truncate long lines to avoid horizontal overflow
        let display = if line.len() > 120 {
            format!(" - {}…", &line[..120])
        } else {
            format!(" - {line}")
        };
        lines.push(Line::from(display));
    }
    Text::from(lines)
}
