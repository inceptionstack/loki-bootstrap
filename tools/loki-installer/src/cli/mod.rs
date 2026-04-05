//! Command-line entrypoints and argument parsing.

pub mod args;
mod commands;
mod output;

use crate::cli::args::{Cli, Command};
use crate::cli::output::print_json_line;
use crate::core::{Planner, RepoAvailabilityCheck, probe_repo_root};
use clap::Parser;
use color_eyre::Result;
use serde_json::json;

pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    let repo_probe = probe_repo_root(cli.repo_path.as_deref());
    if cli.for_agent
        && let Some(event) = &repo_probe.sync_event
    {
        print_json_line(&json!({
            "event": "repo_sync",
            "action": event.action,
            "path": event.path,
            "ref": event.ref_name,
        }))?;
    }

    match cli.command {
        Command::Doctor(args) => {
            let planner = repo_probe
                .root
                .as_deref()
                .map(Planner::from_repo_root)
                .transpose()?;
            let repo_availability = RepoAvailabilityCheck {
                passed: repo_probe.root.is_some(),
                message: repo_probe
                    .root
                    .as_ref()
                    .map(|path| format!("repo available at {}", path.display()))
                    .or_else(|| repo_probe.error.clone())
                    .unwrap_or_else(|| "repo availability unknown".into()),
            };
            commands::doctor::run(args, cli.for_agent, planner, repo_availability).await
        }
        Command::Install(args) => {
            commands::install::run(args, cli.for_agent, required_planner(&repo_probe)?).await
        }
        Command::Plan(args) => {
            commands::plan::run(args, cli.for_agent, required_planner(&repo_probe)?).await
        }
        Command::Resume(args) => {
            commands::resume::run(args, cli.for_agent, required_planner(&repo_probe)?).await
        }
        Command::Uninstall(args) => {
            commands::uninstall::run(args, cli.for_agent, required_planner(&repo_probe)?).await
        }
        Command::Status(args) => {
            commands::status::run(args, cli.for_agent, required_planner(&repo_probe)?).await
        }
    }
}

fn required_planner(repo_probe: &crate::core::RepoProbe) -> Result<Planner> {
    let root = repo_probe.root.as_deref().ok_or_else(|| {
        color_eyre::eyre::eyre!(
            "{}",
            repo_probe
                .error
                .clone()
                .unwrap_or_else(|| "repo root not available".into())
        )
    })?;
    Ok(Planner::from_repo_root(root)?)
}
