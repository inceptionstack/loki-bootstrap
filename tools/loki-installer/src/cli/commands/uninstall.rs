//! `uninstall` subcommand.

use crate::cli::args::UninstallArgs;
use crate::cli::output::{print_human_line, print_json_line};
use crate::core::{Planner, load_latest_session, load_session};
use color_eyre::Result;

const UNSUPPORTED_UNINSTALL_MESSAGE: &str = "Uninstall is not supported yet. Coming soon.";

pub async fn run(args: UninstallArgs, for_agent: bool, planner: Planner) -> Result<()> {
    let mut session = match args.session.as_deref() {
        Some(session_id) => load_session(session_id)?,
        None => load_latest_session()?,
    };
    planner.attach_repo_root(&mut session);

    let supports_uninstall = session
        .plan
        .as_ref()
        .map(|plan| plan.resolved_method.supports_uninstall)
        .unwrap_or(false);

    if supports_uninstall
        && let Err(error) = planner.uninstall(&session).await
        && error.to_string() != UNSUPPORTED_UNINSTALL_MESSAGE
    {
        return Err(error.into());
    }

    if args.json {
        print_json_line(&serde_json::json!({
            "session_id": session.session_id,
            "uninstall": "not_supported",
            "message": UNSUPPORTED_UNINSTALL_MESSAGE
        }))?;
    } else {
        print_human_line(
            for_agent,
            format!(
                "{} Session: {}",
                UNSUPPORTED_UNINSTALL_MESSAGE, session.session_id
            ),
        )?;
    }
    Ok(())
}
