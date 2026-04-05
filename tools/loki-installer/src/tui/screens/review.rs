//! Install plan review screen content.

use crate::tui::app::{AppState, TuiInstallMode};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};

pub fn content(state: &AppState) -> Text<'static> {
    let mut lines = vec![Line::from("Review install plan"), Line::from("")];
    if let Some(plan) = &state.plan {
        lines.push(label_value_line("Pack: ", &plan.resolved_pack.id));
        lines.push(label_value_line("Profile: ", &plan.resolved_profile.id));
        lines.push(label_value_line(
            "Method: ",
            &plan.resolved_method.id.to_string(),
        ));
        lines.push(label_value_line("Region: ", &plan.resolved_region));
        lines.push(label_value_line(
            "Stack name: ",
            &plan.resolved_stack_name.clone().unwrap_or_default(),
        ));
        if state.install_mode == TuiInstallMode::Simple {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "Using recommended defaults. Press A for advanced options.",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Steps:",
            Style::default().fg(Color::Cyan),
        )]));
        for step in &plan.deploy_steps {
            lines.push(Line::from(vec![
                Span::raw(" - "),
                Span::styled(
                    format!("[{}] ", step.phase),
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(step.display_name.clone()),
            ]));
        }
        if !plan.warnings.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "Warnings:",
                Style::default().fg(Color::Cyan),
            )]));
            for warning in &plan.warnings {
                lines.push(Line::from(format!(" ! {}", warning.message)));
            }
        }
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Press Enter to deploy",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]));
    } else {
        lines.push(Line::from("No plan available."));
    }
    Text::from(lines)
}

fn label_value_line(label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(label.to_string(), Style::default().fg(Color::Cyan)),
        Span::styled(
            value.to_string(),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
    ])
}
