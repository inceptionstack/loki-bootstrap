//! Deployment method selection screen content.

use crate::tui::app::AppState;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};

pub fn content(state: &AppState) -> Text<'static> {
    let mut lines = vec![Line::from("Select a deployment method"), Line::from("")];
    for (idx, method) in state.methods.iter().enumerate() {
        let selected = idx == state.request_draft.method_cursor;
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
            Span::styled(format!("{} ({})", method.display_name, method.id), style),
        ]));
        if idx == state.request_draft.method_cursor {
            if let Some(description) = &method.description {
                lines.push(Line::from(vec![Span::styled(
                    format!("    {description}"),
                    Style::default().fg(Color::DarkGray),
                )]));
            }
            lines.push(Line::from(vec![
                Span::styled("    Tools: ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    method.required_tools.join(", "),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }
    }
    Text::from(lines)
}
