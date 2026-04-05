use crate::core::{AdapterError, InstallEvent, InstallEventSink};
use regex::Regex;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc::unbounded_channel;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandSpec {
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    pub(crate) current_dir: Option<String>,
    pub(crate) env: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandOutput {
    pub(crate) status_code: Option<i32>,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

impl CommandOutput {
    pub(crate) fn success(&self) -> bool {
        self.status_code == Some(0)
    }
}

pub(crate) async fn run_command(spec: &CommandSpec) -> Result<CommandOutput, AdapterError> {
    let mut command = build_command(spec);
    let output = command.output().await.map_err(|source| {
        if source.kind() == std::io::ErrorKind::NotFound {
            return AdapterError::Message(format!(
                "{} not found — install {} and re-run the installer",
                spec.program, spec.program
            ));
        }
        AdapterError::Message(format!(
            "Failed to run {} {} — {source}",
            spec.program,
            spec.args.join(" ")
        ))
    })?;

    Ok(CommandOutput {
        status_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    })
}

pub(crate) async fn run_command_streaming(
    spec: &CommandSpec,
    event_sink: &mut dyn InstallEventSink,
) -> Result<CommandOutput, AdapterError> {
    let mut command = build_command(spec);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = spawn_command(command, spec)?;

    let stdout = child.stdout.take().ok_or_else(|| {
        AdapterError::Message(format!("Failed to capture stdout for {}", spec.program))
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        AdapterError::Message(format!("Failed to capture stderr for {}", spec.program))
    })?;

    #[derive(Debug)]
    enum StreamMessage {
        Stdout(String),
        Stderr(String),
        ReadError(String),
    }

    async fn forward_lines<R>(
        reader: R,
        tx: tokio::sync::mpsc::UnboundedSender<StreamMessage>,
        is_stdout: bool,
    ) where
        R: tokio::io::AsyncRead + Unpin + Send + 'static,
    {
        let mut lines = BufReader::new(reader).lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    let message = if is_stdout {
                        StreamMessage::Stdout(line)
                    } else {
                        StreamMessage::Stderr(line)
                    };
                    if tx.send(message).is_err() {
                        return;
                    }
                }
                Ok(None) => return,
                Err(err) => {
                    let _ = tx.send(StreamMessage::ReadError(err.to_string()));
                    return;
                }
            }
        }
    }

    let (tx, mut rx) = unbounded_channel();
    let stdout_task = tokio::spawn(forward_lines(stdout, tx.clone(), true));
    let stderr_task = tokio::spawn(forward_lines(stderr, tx.clone(), false));
    drop(tx);

    let mut stdout_lines = Vec::new();
    let mut stderr_lines = Vec::new();

    while let Some(message) = rx.recv().await {
        match message {
            StreamMessage::Stdout(line) => {
                let line = strip_ansi(&line);
                if !line.is_empty() {
                    event_sink
                        .emit(InstallEvent::LogLine {
                            message: line.clone(),
                        })
                        .await;
                    stdout_lines.push(line);
                }
            }
            StreamMessage::Stderr(line) => {
                let line = strip_ansi(&line);
                if !line.is_empty() {
                    event_sink
                        .emit(InstallEvent::LogLine {
                            message: line.clone(),
                        })
                        .await;
                    stderr_lines.push(line);
                }
            }
            StreamMessage::ReadError(err) => {
                return Err(AdapterError::Message(format!(
                    "Failed to stream {} output — {err}",
                    spec.program
                )));
            }
        }
    }

    let status = child.wait().await.map_err(|source| {
        AdapterError::Message(format!(
            "Failed to wait for {} {} — {source}",
            spec.program,
            spec.args.join(" ")
        ))
    })?;

    stdout_task
        .await
        .map_err(|source| AdapterError::Message(format!("stdout task failed — {source}")))?;
    stderr_task
        .await
        .map_err(|source| AdapterError::Message(format!("stderr task failed — {source}")))?;

    Ok(CommandOutput {
        status_code: status.code(),
        stdout: stdout_lines.join("\n").trim().to_string(),
        stderr: stderr_lines.join("\n").trim().to_string(),
    })
}

fn strip_ansi(input: &str) -> String {
    let re = Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap();
    let cleaned = re.replace_all(input, "");
    cleaned
        .replace(['│', '╵', '╷'], "")
        .replace('\r', "")
        .trim()
        .to_string()
}

pub(crate) fn spawn_child(spec: &CommandSpec) -> Result<tokio::process::Child, AdapterError> {
    spawn_command(build_command(spec), spec)
}

pub(crate) fn resolve_repo_path_from(
    repo_root: Option<&str>,
    path: &str,
) -> Result<String, AdapterError> {
    let candidate = PathBuf::from(path);
    if candidate.is_absolute() && candidate.exists() {
        return Ok(candidate.display().to_string());
    }

    if let Some(root) = repo_root {
        let joined = Path::new(root).join(&candidate);
        if joined.exists() {
            return Ok(joined.display().to_string());
        }
    }

    let cwd = std::env::current_dir().map_err(|source| {
        AdapterError::Message(format!(
            "Failed to read current working directory — {source}"
        ))
    })?;

    let direct = cwd.join(&candidate);
    if direct.exists() {
        return Ok(direct.display().to_string());
    }

    for ancestor in cwd.ancestors() {
        let repo_candidate = ancestor.join(&candidate);
        if repo_candidate.exists() {
            return Ok(repo_candidate.display().to_string());
        }
    }

    Err(AdapterError::Message(format!(
        "Required installer asset not found at {path} — verify the repo checkout is complete"
    )))
}

fn build_command(spec: &CommandSpec) -> Command {
    let mut command = Command::new(&spec.program);
    command.args(&spec.args);
    command.envs(&spec.env);
    command.kill_on_drop(true);
    if let Some(current_dir) = &spec.current_dir {
        command.current_dir(Path::new(current_dir));
    }
    command
}

fn spawn_command(
    mut command: Command,
    spec: &CommandSpec,
) -> Result<tokio::process::Child, AdapterError> {
    command.spawn().map_err(|source| {
        if source.kind() == std::io::ErrorKind::NotFound {
            return AdapterError::Message(format!(
                "{} not found — install {} and re-run the installer",
                spec.program, spec.program
            ));
        }
        AdapterError::Message(format!(
            "Failed to start {} {} — {source}",
            spec.program,
            spec.args.join(" ")
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::strip_ansi;

    #[test]
    fn strip_ansi_removes_escape_sequences_and_box_drawing() {
        assert_eq!(
            strip_ansi("\u{1b}[31m│ Error ╵ details ╷\u{1b}[0m"),
            "Error  details"
        );
        assert_eq!(strip_ansi("   \u{1b}[32mok\u{1b}[0m   "), "ok");
    }
}
