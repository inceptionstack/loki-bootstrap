//! Welcome screen content.

use crate::tui::app::AppState;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};

pub fn content(_state: &AppState) -> Text<'static> {
    Text::from(vec![
        Line::from(vec![Span::styled(
            " _     ___  _  ___   ___ _   _ ____ _____  _    _     _     _____ ____  ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "| |   / _ \\| |/ / | |_ _| \\ | / ___|_   _|/ \\  | |   | |   | ____|  _ \\ ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "| |  | | | | ' /| |  | ||  \\| \\___ \\ | | / _ \\ | |   | |   |  _| | |_) |",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "| |__| |_| | . \\| |  | || |\\  |___) || |/ ___ \\| |___| |___| |___|  _ < ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "|_____\\___/|_|\\_\\_| |___|_| \\_|____/ |_/_/   \\_\\_____|_____|_____|_| \\_\\",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Version: ", Style::default().fg(Color::DarkGray)),
            Span::styled(env!("CARGO_PKG_VERSION"), Style::default().fg(Color::Green)),
        ]),
        Line::from(""),
        Line::from("Installer V2 deploys Loki packs with a shared core for CLI and TUI."),
        Line::from(""),
        Line::from("Choose install mode:"),
        Line::from(vec![Span::styled(
            "[S] Simple install (recommended)",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from("[A] Advanced install"),
        Line::from(""),
        Line::from("Enter also starts Simple mode."),
    ])
}
