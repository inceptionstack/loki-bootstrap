//! Profile selection screen content.

use crate::tui::app::AppState;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};

pub fn content(state: &AppState) -> Text<'static> {
    let mut lines = vec![Line::from("Select a profile"), Line::from("")];
    for (idx, profile) in state.profiles.iter().enumerate() {
        let selected = idx == state.request_draft.profile_cursor;
        let style = if selected {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::DIM)
        };
        lines.push(Line::from(vec![
            Span::styled(if selected { "> " } else { "  " }, style),
            Span::styled(format!("{} ({})", profile.display_name, profile.id), style),
        ]));
        if idx == state.request_draft.profile_cursor
            && let Some(description) = &profile.description
        {
            lines.push(Line::from(vec![Span::styled(
                format!("    {description}"),
                Style::default().fg(Color::DarkGray),
            )]));
        }
    }
    Text::from(lines)
}
