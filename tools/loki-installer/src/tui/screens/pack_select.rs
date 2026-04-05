//! Pack selection screen content.

use crate::tui::app::AppState;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};

pub fn content(state: &AppState) -> Text<'static> {
    let mut lines = vec![Line::from("Select a pack")];
    lines.push(Line::from(""));
    for (idx, pack) in state.packs.iter().enumerate() {
        let selected = idx == state.request_draft.pack_cursor;
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
            Span::styled(format!("{} ({})", pack.display_name, pack.id), style),
        ]));
        if idx == state.request_draft.pack_cursor
            && let Some(description) = &pack.description
        {
            lines.push(Line::from(vec![Span::styled(
                format!("    {description}"),
                Style::default().fg(Color::DarkGray),
            )]));
        }
    }
    Text::from(lines)
}
