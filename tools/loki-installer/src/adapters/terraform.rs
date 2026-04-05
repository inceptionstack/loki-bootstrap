//! Terraform deployment adapter.

use crate::adapters::support::{
    CommandOutput, CommandSpec, resolve_repo_path_from, run_command, run_command_streaming,
};
use crate::core::{
    AdapterError, AdapterPlan, AdapterValidationError, ApplyResult, DeployAction, DeployAdapter,
    DeployMethodId, DeployStatus, DeployStep, InstallEvent, InstallEventSink, InstallPhase,
    InstallPlan, InstallRequest, InstallSession, MethodManifest, PackManifest, PlanWarning,
    PostInstallStep, PrerequisiteCheck, PrerequisiteKind, ProfileManifest, UninstallResult,
    update_session_phase,
};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

pub struct TerraformAdapter;
const DEFAULT_TERRAFORM_VERSION: &str = "1.12.1";
const TERRAFORM_PLUGIN_CACHE_DIR: &str = "/tmp/terraform-plugin-cache";
const SSM_SESSION_DOCUMENT_NAME: &str = "Loki-Session";
const TERRAFORM_PATH_ENV: &str = "PATH";
const TERRAFORM_VERSION_ENV: &str = "LOKI_INSTALLER_TERRAFORM_VERSION";
const BOOTSTRAP_POLL_ATTEMPTS: u32 = 60;
const BOOTSTRAP_POLL_INTERVAL: Duration = Duration::from_secs(10);

#[derive(Debug, Clone)]
struct TerraformContext {
    terraform_bin: String,
    working_dir: String,
    plan_file: String,
    region: String,
    pack: String,
    profile: String,
    environment_name: String,
    tf_vars: Vec<(String, String)>,
}

#[derive(Debug, Deserialize)]
struct TerraformOutputValue {
    value: serde_json::Value,
}

#[async_trait::async_trait]
impl DeployAdapter for TerraformAdapter {
    fn method_id(&self) -> DeployMethodId {
        DeployMethodId::Terraform
    }

    fn validate_request(
        &self,
        _request: &InstallRequest,
        _pack: &PackManifest,
        _profile: Option<&ProfileManifest>,
        _method: &MethodManifest,
    ) -> Result<(), AdapterValidationError> {
        Ok(())
    }

    async fn build_plan(
        &self,
        request: &InstallRequest,
        pack: &PackManifest,
        profile: &ProfileManifest,
        _method: &MethodManifest,
    ) -> Result<AdapterPlan, AdapterError> {
        let mut adapter_options = BTreeMap::new();
        adapter_options.insert("working_dir".into(), "deploy/terraform".into());
        adapter_options.insert("pack".into(), pack.id.clone());
        adapter_options.insert("profile".into(), profile.id.clone());
        if let Some(region) = &request.region {
            adapter_options.insert("region".into(), region.clone());
        }

        Ok(AdapterPlan {
            prerequisites: vec![PrerequisiteCheck {
                id: "terraform_cli".into(),
                display_name: "Terraform CLI available".into(),
                kind: PrerequisiteKind::MethodToolingPresent,
                required: true,
                remediation: Some("Install terraform and re-run the installer.".into()),
            }],
            deploy_steps: vec![
                DeployStep {
                    id: "terraform-init".into(),
                    phase: InstallPhase::PrepareDeployment,
                    display_name: "Initialize Terraform".into(),
                    action: DeployAction::RunCommand {
                        program: "terraform".into(),
                        args: vec!["init".into(), "-input=false".into()],
                    },
                },
                DeployStep {
                    id: "terraform-plan".into(),
                    phase: InstallPhase::PlanDeployment,
                    display_name: "Plan Terraform changes".into(),
                    action: DeployAction::RunCommand {
                        program: "terraform".into(),
                        args: vec!["plan".into(), "-input=false".into()],
                    },
                },
                DeployStep {
                    id: "terraform-apply".into(),
                    phase: InstallPhase::ApplyDeployment,
                    display_name: "Apply Terraform changes".into(),
                    action: DeployAction::RunCommand {
                        program: "terraform".into(),
                        args: vec![
                            "apply".into(),
                            "-input=false".into(),
                            "-auto-approve".into(),
                        ],
                    },
                },
                DeployStep {
                    id: "terraform-health".into(),
                    phase: InstallPhase::PostInstall,
                    display_name: "Capture Terraform outputs".into(),
                    action: DeployAction::VerifyInstanceHealth,
                },
                DeployStep {
                    id: "bootstrap-wait".into(),
                    phase: InstallPhase::PostInstall,
                    display_name: "Wait for instance bootstrap".into(),
                    action: DeployAction::VerifyInstanceHealth,
                },
                DeployStep {
                    id: "ssm-session-doc".into(),
                    phase: InstallPhase::PostInstall,
                    display_name: "Ensure SSM session document".into(),
                    action: DeployAction::EmitInstructions,
                },
            ],
            adapter_options,
            warnings: vec![PlanWarning {
                code: "terraform_async_bootstrap".into(),
                message: "Terraform reports success before EC2 bootstrap finishes; verify health afterward.".into(),
            }],
            post_install_steps: vec![PostInstallStep {
                id: "terraform-output".into(),
                display_name: "Inspect Terraform outputs".into(),
                instruction: "terraform -chdir=deploy/terraform output".into(),
            }],
        })
    }

    async fn apply(
        &self,
        plan: &InstallPlan,
        session: &mut InstallSession,
        event_sink: &mut dyn InstallEventSink,
    ) -> Result<ApplyResult, AdapterError> {
        apply_terraform(plan, session, event_sink).await
    }

    async fn resume(
        &self,
        session: &mut InstallSession,
        event_sink: &mut dyn InstallEventSink,
    ) -> Result<ApplyResult, AdapterError> {
        let plan = session.plan.clone().ok_or(AdapterError::NotResumable)?;
        apply_terraform(&plan, session, event_sink).await
    }

    async fn uninstall(
        &self,
        _session: &InstallSession,
        _event_sink: &mut dyn InstallEventSink,
    ) -> Result<UninstallResult, AdapterError> {
        Err(AdapterError::Message(
            "Uninstall is not supported yet. Coming soon.".into(),
        ))
    }

    async fn status(&self, session: &InstallSession) -> Result<DeployStatus, AdapterError> {
        let plan = session.plan.as_ref();
        Ok(DeployStatus {
            deployed: session.phase == InstallPhase::PostInstall,
            pack: session.request.pack.clone(),
            profile: plan
                .map(|plan| plan.resolved_profile.id.clone())
                .or_else(|| session.request.profile.clone())
                .unwrap_or_default(),
            method: DeployMethodId::Terraform,
            region: plan
                .map(|plan| plan.resolved_region.clone())
                .or_else(|| session.request.region.clone()),
            stack_name: plan
                .and_then(|plan| plan.resolved_stack_name.clone())
                .or_else(|| session.request.stack_name.clone()),
            stack_status: session
                .artifacts
                .get("stack_status")
                .cloned()
                .or(Some("terraform_applied".into())),
            instance_health: session.artifacts.get("instance_health").cloned(),
            last_updated_at: session.updated_at,
        })
    }
}

impl TerraformContext {
    fn from_plan(plan: &InstallPlan, terraform_bin: String) -> Result<Self, AdapterError> {
        let working_dir = resolve_repo_path_from(
            plan.adapter_options.get("repo_root").map(String::as_str),
            plan.adapter_options
                .get("working_dir")
                .map(String::as_str)
                .unwrap_or("deploy/terraform"),
        )?;
        let environment_name = plan.resolved_stack_name.clone().unwrap_or_else(|| {
            // 1. Try reading from existing terraform state
            if let Some(name) = read_env_name_from_tf_state(&working_dir) {
                return name;
            }
            // 2. Try reading from .loki-env (persisted from previous install)
            let env_file = Path::new(&working_dir).join(".loki-env");
            if let Ok(contents) = fs::read_to_string(&env_file) {
                for line in contents.lines() {
                    if let Some(name) = line.strip_prefix("ENVIRONMENT_NAME=") {
                        let name = name.trim();
                        if !name.is_empty() {
                            return name.to_string();
                        }
                    }
                }
            }
            // 3. Generate a new unique name
            let suffix = &uuid::Uuid::new_v4().to_string()[..8];
            format!("loki-{}-{suffix}", plan.resolved_pack.id)
        });
        // Persist environment name for future re-installs
        let env_file = Path::new(&working_dir).join(".loki-env");
        let _ = fs::write(&env_file, format!("ENVIRONMENT_NAME={environment_name}\n"));

        Ok(Self {
            terraform_bin,
            plan_file: Path::new(&working_dir).join("tfplan").display().to_string(),
            working_dir,
            region: plan.resolved_region.clone(),
            pack: plan.resolved_pack.id.clone(),
            profile: plan.resolved_profile.id.clone(),
            environment_name,
            tf_vars: plan
                .adapter_options
                .iter()
                .filter_map(|(key, value)| {
                    adapter_option_to_tf_var(key)
                        .map(|var_name| (var_name.to_string(), value.clone()))
                })
                .collect(),
        })
    }
}

async fn apply_terraform(
    plan: &InstallPlan,
    session: &mut InstallSession,
    event_sink: &mut dyn InstallEventSink,
) -> Result<ApplyResult, AdapterError> {
    if session.phase == InstallPhase::PostInstall {
        return Ok(completed_apply_result(plan, session));
    }

    let terraform_bin = ensure_terraform_installed(event_sink)?;
    let mut context = TerraformContext::from_plan(plan, terraform_bin)?;
    if !Path::new(&context.working_dir).exists() {
        return Err(AdapterError::Message(format!(
            "Terraform working directory not found at {} — verify the repo checkout includes deploy/terraform",
            context.working_dir
        )));
    }

    let mut artifacts = BTreeMap::new();
    artifacts.insert("stack_name".into(), context.environment_name.clone());

    for step in &plan.deploy_steps {
        if phase_is_past(session.phase, step.phase) {
            event_sink
                .emit(InstallEvent::LogLine {
                    message: format!(
                        "Skipping {} on resume; phase {} already completed",
                        step.display_name, step.phase
                    ),
                })
                .await;
            continue;
        }

        emit_step_started(session, event_sink, step).await;

        match step.id.as_str() {
            "terraform-init" => {
                prepare_terraform_init_context(&mut context, event_sink).await?;
                let output = run_command(&build_terraform_init_command(&context)).await?;
                ensure_terraform_success(
                    &output,
                    "Terraform init failed — run terraform init manually in deploy/terraform for details",
                )?;
                record_command_artifact(
                    "terraform_init_output",
                    output,
                    &mut artifacts,
                    event_sink,
                )
                .await;
            }
            "terraform-plan" => {
                let output =
                    run_command(&build_terraform_plan_command(&context, &context.tf_vars)).await?;
                ensure_terraform_success(
                    &output,
                    "Terraform plan failed — fix the reported variable or provider issue and retry",
                )?;
                record_command_artifact(
                    "terraform_plan_output",
                    output,
                    &mut artifacts,
                    event_sink,
                )
                .await;
            }
            "terraform-apply" => {
                let output =
                    run_command_streaming(&build_terraform_apply_command(&context), event_sink)
                        .await?;
                let apply_output = if output.success() {
                    output
                } else {
                    let combined_output = format!("{}\n{}", output.stderr, output.stdout);
                    let is_existing_resource_error = combined_output
                        .contains("EntityAlreadyExists")
                        || combined_output
                            .to_ascii_lowercase()
                            .contains("already exists");
                    let existing_resources = parse_existing_resources(&combined_output);

                    if is_existing_resource_error && !existing_resources.is_empty() {
                        let retry_tf_vars =
                            with_tf_var(&context.tf_vars, "enable_satellite_services", "false");
                        for (resource_address, import_id) in existing_resources {
                            event_sink
                                .emit(InstallEvent::LogLine {
                                    message: format!(
                                        "Auto-importing pre-existing resource: {resource_address} ({import_id})"
                                    ),
                                })
                                .await;
                            run_terraform_import(&context, &resource_address, &import_id).await?;
                        }

                        let retry_plan =
                            run_command(&build_terraform_plan_command(&context, &retry_tf_vars))
                                .await?;
                        ensure_terraform_success(
                            &retry_plan,
                            "Terraform plan failed — fix the reported variable or provider issue and retry",
                        )?;
                        record_command_artifact(
                            "terraform_plan_output",
                            retry_plan,
                            &mut artifacts,
                            event_sink,
                        )
                        .await;

                        let retry_apply = run_command_streaming(
                            &build_terraform_apply_command(&context),
                            event_sink,
                        )
                        .await?;
                        ensure_terraform_success(
                            &retry_apply,
                            "Terraform apply failed — inspect the Terraform output for the rejected AWS resource or permission",
                        )?;
                        retry_apply
                    } else {
                        ensure_terraform_success(
                            &output,
                            "Terraform apply failed — inspect the Terraform output for the rejected AWS resource or permission",
                        )?;
                        output
                    }
                };
                record_command_artifact(
                    "terraform_apply_output",
                    apply_output,
                    &mut artifacts,
                    event_sink,
                )
                .await;
            }
            "terraform-health" => {
                let output = run_command(&build_terraform_output_command(&context)).await?;
                ensure_terraform_success(
                    &output,
                    "Terraform output failed — run terraform output -json in deploy/terraform",
                )?;
                for (key, value) in parse_terraform_outputs(&output.stdout)? {
                    artifacts.insert(key.clone(), value.clone());
                    event_sink
                        .emit(InstallEvent::ArtifactRecorded { key, value })
                        .await;
                }
                artifacts.insert("stack_status".into(), "terraform_applied".into());
                artifacts.insert("instance_health".into(), "unknown".into());
            }
            "bootstrap-wait" => {
                wait_for_bootstrap(&context, &artifacts, event_sink).await?;
                artifacts.insert("instance_health".into(), "ready".into());
            }
            "ssm-session-doc" => {
                ensure_ssm_session_document(&context.region, &mut artifacts, event_sink).await;
            }
            _ => {}
        }

        emit_step_finished(event_sink, step, "completed").await;
    }

    update_session_phase(session, InstallPhase::PostInstall);
    session.artifacts.extend(artifacts.clone());

    Ok(ApplyResult {
        final_phase: InstallPhase::PostInstall,
        artifacts,
        post_install_steps: plan.post_install_steps.clone(),
    })
}

fn build_terraform_init_command(context: &TerraformContext) -> CommandSpec {
    CommandSpec {
        program: context.terraform_bin.clone(),
        args: vec![
            format!("-chdir={}", context.working_dir),
            "init".into(),
            "-no-color".into(),
            "-input=false".into(),
        ],
        current_dir: None,
        env: terraform_init_env(),
    }
}

fn build_terraform_plan_command(
    context: &TerraformContext,
    tf_vars: &[(String, String)],
) -> CommandSpec {
    let mut args = vec![
        format!("-chdir={}", context.working_dir),
        "plan".into(),
        "-no-color".into(),
        "-input=false".into(),
        format!("-out={}", context.plan_file),
        format!("-var=aws_region={}", context.region),
        format!("-var=pack_name={}", context.pack),
        format!("-var=profile_name={}", context.profile),
        format!("-var=environment_name={}", context.environment_name),
    ];
    for (name, value) in tf_vars {
        args.push(format!("-var={name}={value}"));
    }

    CommandSpec {
        program: context.terraform_bin.clone(),
        args,
        current_dir: None,
        env: BTreeMap::new(),
    }
}

fn adapter_option_to_tf_var(key: &str) -> Option<&'static str> {
    match key {
        "model" => Some("default_model"),
        "port" => Some("openclaw_gateway_port"),
        "bedrockify_port" => Some("bedrockify_port"),
        "embed_model" => Some("embed_model"),
        "hermes_model" => Some("hermes_model"),
        "haiku_model" => Some("haiku_model"),
        "sandbox_name" => Some("sandbox_name"),
        "enable_satellite_services" => Some("enable_satellite_services"),
        "working_dir" | "pack" | "profile" | "region" | "workspace" | "repo_root" => None,
        _ => None,
    }
}

fn build_terraform_apply_command(context: &TerraformContext) -> CommandSpec {
    CommandSpec {
        program: context.terraform_bin.clone(),
        args: vec![
            format!("-chdir={}", context.working_dir),
            "apply".into(),
            "-no-color".into(),
            "-input=false".into(),
            "-auto-approve".into(),
            context.plan_file.clone(),
        ],
        current_dir: None,
        env: BTreeMap::new(),
    }
}

fn with_tf_var(tf_vars: &[(String, String)], name: &str, value: &str) -> Vec<(String, String)> {
    let mut updated = tf_vars.to_vec();
    if let Some((_, existing_value)) = updated.iter_mut().find(|(key, _)| key == name) {
        *existing_value = value.to_string();
    } else {
        updated.push((name.to_string(), value.to_string()));
    }
    updated
}

fn build_terraform_output_command(context: &TerraformContext) -> CommandSpec {
    CommandSpec {
        program: context.terraform_bin.clone(),
        args: vec![
            format!("-chdir={}", context.working_dir),
            "output".into(),
            "-json".into(),
            "-no-color".into(),
        ],
        current_dir: None,
        env: BTreeMap::new(),
    }
}

fn build_terraform_import_command(
    context: &TerraformContext,
    resource_address: &str,
    import_id: &str,
) -> CommandSpec {
    CommandSpec {
        program: context.terraform_bin.clone(),
        args: vec![
            format!("-chdir={}", context.working_dir),
            "import".into(),
            "-no-color".into(),
            resource_address.into(),
            import_id.into(),
        ],
        current_dir: None,
        env: BTreeMap::new(),
    }
}

fn terraform_init_env() -> BTreeMap<String, String> {
    BTreeMap::from([(
        "TF_PLUGIN_CACHE_DIR".into(),
        TERRAFORM_PLUGIN_CACHE_DIR.into(),
    )])
}

async fn prepare_terraform_init_context(
    context: &mut TerraformContext,
    event_sink: &mut dyn InstallEventSink,
) -> Result<(), AdapterError> {
    fs::create_dir_all(TERRAFORM_PLUGIN_CACHE_DIR).map_err(|source| {
        AdapterError::Message(format!(
            "Failed to create terraform plugin cache directory {TERRAFORM_PLUGIN_CACHE_DIR} — {source}"
        ))
    })?;

    if let Some(available_mb) = available_disk_space_mb(Path::new(&context.working_dir))?
        && available_mb < 600
    {
        emit_sync_log(
            event_sink,
            &format!(
                "Low disk space ({available_mb}MB available) — Terraform providers need ~500MB"
            ),
        )?;
        if is_cloudshell() {
            emit_sync_log(
                event_sink,
                "CloudShell detected — moving Terraform workdir to /tmp",
            )?;
            relocate_terraform_workdir_to_tmp(context)?;
            emit_sync_log(
                event_sink,
                &format!("Working from: {}", context.working_dir),
            )?;
        } else {
            emit_sync_log(
                event_sink,
                "You may run out of disk space. Consider freeing space or using /tmp.",
            )?;
        }
    }

    emit_sync_log(
        event_sink,
        &format!("Plugin cache: {TERRAFORM_PLUGIN_CACHE_DIR}"),
    )?;
    Ok(())
}

fn available_disk_space_mb(path: &Path) -> Result<Option<u64>, AdapterError> {
    let output = Command::new("df")
        .args(["-Pm"])
        .arg(path)
        .output()
        .map_err(|source| {
            AdapterError::Message(format!(
                "Failed to check available disk space for {} — {source}",
                path.display()
            ))
        })?;
    if !output.status.success() {
        return Ok(None);
    }

    Ok(parse_df_available_mb(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

fn parse_df_available_mb(raw: &str) -> Option<u64> {
    raw.lines()
        .nth(1)
        .and_then(|line| line.split_whitespace().nth(3))
        .and_then(|value| value.parse::<u64>().ok())
}

fn is_cloudshell() -> bool {
    std::env::var_os("AWS_CLOUDSHELL_HOME").is_some()
        || std::env::var("AWS_EXECUTION_ENV")
            .map(|value| value.contains("CloudShell"))
            .unwrap_or(false)
        || std::env::var("USER")
            .map(|value| value == "cloudshell-user")
            .unwrap_or(false)
}

fn relocate_terraform_workdir_to_tmp(context: &mut TerraformContext) -> Result<(), AdapterError> {
    let relocated_dir = std::env::temp_dir().join(format!("loki-terraform-{}", std::process::id()));
    fs::create_dir_all(&relocated_dir).map_err(|source| {
        AdapterError::Message(format!(
            "Failed to create temporary terraform workdir {} — {source}",
            relocated_dir.display()
        ))
    })?;

    let output = Command::new("cp")
        .args(["-a"])
        .arg(format!("{}/.", context.working_dir))
        .arg(&relocated_dir)
        .output()
        .map_err(|source| {
            AdapterError::Message(format!(
                "Failed to copy terraform workdir to {} — {source}",
                relocated_dir.display()
            ))
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(AdapterError::Message(format!(
            "Failed to copy terraform workdir to {} — {stderr}",
            relocated_dir.display()
        )));
    }

    context.working_dir = relocated_dir.display().to_string();
    context.plan_file = relocated_dir.join("tfplan").display().to_string();
    Ok(())
}

async fn run_terraform_import(
    context: &TerraformContext,
    resource_address: &str,
    import_id: &str,
) -> Result<CommandOutput, AdapterError> {
    let output = run_command(&build_terraform_import_command(
        context,
        resource_address,
        import_id,
    ))
    .await?;
    ensure_terraform_success(
        &output,
        "Terraform import failed — inspect the Terraform output for the rejected AWS resource or permission",
    )?;
    Ok(output)
}

fn ensure_terraform_installed(
    event_sink: &mut dyn InstallEventSink,
) -> Result<String, AdapterError> {
    if let Some(terraform_bin) = resolve_existing_terraform_binary() {
        return Ok(terraform_bin);
    }

    emit_sync_log(event_sink, "Terraform not found — installing...")?;

    let version = std::env::var(TERRAFORM_VERSION_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_TERRAFORM_VERSION.to_string());
    let (os, arch) = terraform_platform()?;
    let install_path = terraform_install_path()?;
    if let Some(parent) = install_path.parent() {
        fs::create_dir_all(parent).map_err(|source| {
            AdapterError::Message(format!(
                "Failed to create terraform install directory {} — {source}",
                parent.display()
            ))
        })?;
    }

    let url = format!(
        "https://releases.hashicorp.com/terraform/{version}/terraform_{version}_{os}_{arch}.zip"
    );
    let archive_path = terraform_archive_path(&version);
    download_terraform_archive(&url, &archive_path)?;
    extract_terraform_binary(&archive_path, &install_path)?;
    prepend_local_bin_to_path(install_path.parent())?;
    emit_sync_log(
        event_sink,
        &format!("Terraform installed successfully (version {version})"),
    )?;
    let _ = fs::remove_file(&archive_path);

    Ok(install_path.display().to_string())
}

fn resolve_existing_terraform_binary() -> Option<String> {
    if let Some(path) = find_command_on_path("terraform")
        && Command::new(&path)
            .arg("version")
            .status()
            .is_ok_and(|status| status.success())
    {
        return Some(path.display().to_string());
    }

    None
}

fn terraform_platform() -> Result<(&'static str, &'static str), AdapterError> {
    let os = match std::env::consts::OS {
        "linux" => "linux",
        "macos" => "darwin",
        other => {
            return Err(AdapterError::Message(format!(
                "Terraform auto-install is unsupported on OS {other}"
            )));
        }
    };

    let arch = match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        other => {
            return Err(AdapterError::Message(format!(
                "Terraform auto-install is unsupported on architecture {other}"
            )));
        }
    };

    Ok((os, arch))
}

fn terraform_install_path() -> Result<PathBuf, AdapterError> {
    let home = std::env::var_os("HOME").ok_or_else(|| {
        AdapterError::Message(
            "HOME is not set, so the installer cannot place terraform in ~/.local/bin".into(),
        )
    })?;
    let primary = PathBuf::from(&home).join(".local/bin");
    // Fall back to /tmp/.local/bin when ~/.local/bin is on a read-only or
    // space-constrained filesystem (e.g. CloudShell).
    let dir = if can_write_to(&primary) {
        primary
    } else {
        PathBuf::from("/tmp/.local/bin")
    };
    fs::create_dir_all(&dir).map_err(|e| {
        AdapterError::Message(format!(
            "Failed to create directory {} — {e}",
            dir.display()
        ))
    })?;
    Ok(dir.join("terraform"))
}

fn read_env_name_from_tf_state(working_dir: &str) -> Option<String> {
    let state_path = Path::new(working_dir).join("terraform.tfstate");
    let contents = fs::read_to_string(&state_path).ok()?;
    // Look for environment_name in the state's root module outputs or resource attributes
    // The IAM role name pattern is "${environment_name}-role"
    for line in contents.lines() {
        let line = line.trim();
        if line.contains("\"name\"") && line.contains("-role\"") {
            // Extract name like "loki-openclaw-abc12345-role"
            if let (Some(start), Some(end)) = (line.find('"'), line.rfind("-role\"")) {
                let candidate = &line[start + 1..end];
                if candidate.starts_with("loki-") {
                    return Some(candidate.to_string());
                }
            }
        }
    }
    // Fallback: try terraform output
    let output = Command::new("terraform")
        .args(["-chdir", working_dir, "output", "-raw", "environment_name"])
        .output()
        .ok()?;
    if output.status.success() {
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !name.is_empty() {
            return Some(name);
        }
    }
    None
}

fn can_write_to(dir: &Path) -> bool {
    if fs::create_dir_all(dir).is_err() {
        return false;
    }
    let probe = dir.join(".loki-probe");
    match fs::write(&probe, b"ok") {
        Ok(_) => {
            let _ = fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

fn terraform_archive_path(version: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "loki-installer-terraform-{version}-{}.zip",
        std::process::id()
    ))
}

fn download_terraform_archive(url: &str, archive_path: &Path) -> Result<(), AdapterError> {
    let output = Command::new("curl")
        .args(["-fsSL", url, "-o"])
        .arg(archive_path)
        .output()
        .map_err(|source| {
            AdapterError::Message(format!(
                "Failed to start curl while downloading terraform from {url} — {source}"
            ))
        })?;
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(AdapterError::Message(format!(
        "Failed to download terraform archive from {url} — {stderr}"
    )))
}

fn extract_terraform_binary(archive_path: &Path, install_path: &Path) -> Result<(), AdapterError> {
    let parent = install_path
        .parent()
        .ok_or_else(|| AdapterError::Message("install_path has no parent".into()))?;
    let output = Command::new("unzip")
        .args(["-o", "-j"])
        .arg(archive_path)
        .arg("terraform")
        .arg("-d")
        .arg(parent)
        .output()
        .map_err(|source| {
            AdapterError::Message(format!(
                "Failed to run unzip while extracting terraform to {} — {source}",
                install_path.display()
            ))
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(AdapterError::Message(format!(
            "Failed to extract terraform binary to {} — {stderr}",
            install_path.display()
        )));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(install_path)
            .map_err(|e| {
                AdapterError::Message(format!("Failed to read terraform permissions — {e}"))
            })?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(install_path, perms)
            .map_err(|e| AdapterError::Message(format!("Failed to chmod terraform — {e}")))?;
    }
    Ok(())
}

fn find_command_on_path(program: &str) -> Option<PathBuf> {
    std::env::var_os(TERRAFORM_PATH_ENV).and_then(|paths| {
        std::env::split_paths(&paths).find_map(|path| {
            let candidate = path.join(program);
            if candidate.is_file() {
                Some(candidate)
            } else {
                None
            }
        })
    })
}

fn prepend_local_bin_to_path(local_bin_dir: Option<&Path>) -> Result<(), AdapterError> {
    let Some(local_bin_dir) = local_bin_dir else {
        return Ok(());
    };

    let current_path = std::env::var_os(TERRAFORM_PATH_ENV).unwrap_or_default();
    let mut paths: Vec<PathBuf> = std::env::split_paths(&current_path).collect();
    if paths.iter().any(|path| path == local_bin_dir) {
        return Ok(());
    }

    paths.insert(0, local_bin_dir.to_path_buf());
    let joined = std::env::join_paths(paths).map_err(|source| {
        AdapterError::Message(format!(
            "Failed to update PATH for terraform install — {source}"
        ))
    })?;
    // Safety: the installer updates PATH within its own process so subsequent child processes can
    // discover the freshly installed terraform binary.
    unsafe {
        std::env::set_var(TERRAFORM_PATH_ENV, joined);
    }
    Ok(())
}

fn emit_sync_log(event_sink: &mut dyn InstallEventSink, message: &str) -> Result<(), AdapterError> {
    futures::executor::block_on(event_sink.emit(InstallEvent::LogLine {
        message: message.to_string(),
    }));
    Ok(())
}

async fn wait_for_bootstrap(
    context: &TerraformContext,
    artifacts: &BTreeMap<String, String>,
    event_sink: &mut dyn InstallEventSink,
) -> Result<(), AdapterError> {
    let instance_id = artifacts.get("instance_id").cloned().ok_or_else(|| {
        AdapterError::Message("Terraform output did not include instance_id".into())
    })?;

    emit_sync_log(event_sink, "Waiting for Loki to bootstrap (~10 minutes)...")?;
    if let Some(public_ip) = artifacts.get("public_ip") {
        emit_sync_log(
            event_sink,
            &format!("Instance: {instance_id} | IP: {public_ip}"),
        )?;
    } else {
        emit_sync_log(event_sink, &format!("Instance: {instance_id}"))?;
    }

    for parameter in ["/loki/setup-status", "/loki/setup-step", "/loki/setup-log"] {
        let _ = run_command(&aws_ssm_command(
            &context.region,
            &["delete-parameter", "--name", parameter],
        ))
        .await;
    }

    for attempt in 1..=BOOTSTRAP_POLL_ATTEMPTS {
        let setup_status = get_ssm_parameter(&context.region, "/loki/setup-status").await?;
        if setup_status.as_deref() == Some("FAILED") {
            let failed_step = get_ssm_parameter(&context.region, "/loki/setup-step")
                .await?
                .unwrap_or_else(|| "unknown step".into());
            let fail_log = get_ssm_parameter(&context.region, "/loki/setup-log")
                .await?
                .unwrap_or_default();

            emit_sync_log(event_sink, "Bootstrap FAILED")?;
            emit_sync_log(event_sink, &format!("Step: {failed_step}"))?;
            if !fail_log.is_empty() {
                emit_sync_log(event_sink, "Last log output:")?;
                for line in fail_log.lines() {
                    emit_sync_log(event_sink, &format!("  {line}"))?;
                }
            }
            return Err(AdapterError::Message(format!(
                "Bootstrap failed during {failed_step}"
            )));
        }

        if bootstrap_done(&context.region, &instance_id).await? {
            emit_sync_log(event_sink, "Loki is ready!")?;
            return Ok(());
        }

        if let Some(current_step) = get_ssm_parameter(&context.region, "/loki/setup-step").await?
            && !current_step.is_empty()
        {
            emit_sync_log(
                event_sink,
                &format!("Bootstrapping: {current_step} ({attempt}/{BOOTSTRAP_POLL_ATTEMPTS})"),
            )?;
        }

        if attempt < BOOTSTRAP_POLL_ATTEMPTS {
            tokio::time::sleep(BOOTSTRAP_POLL_INTERVAL).await;
        }
    }

    Err(AdapterError::Message(
        "Timed out waiting for bootstrap to finish after 10 minutes".into(),
    ))
}

async fn bootstrap_done(region: &str, instance_id: &str) -> Result<bool, AdapterError> {
    let Some(command_id) = send_ssm_shell_command(
        region,
        instance_id,
        "test -f /tmp/loki-bootstrap-done && echo READY || echo WAITING",
    )
    .await?
    else {
        return Ok(false);
    };

    tokio::time::sleep(Duration::from_secs(5)).await;
    let output = get_ssm_command_output(region, instance_id, &command_id).await?;
    Ok(output.contains("READY"))
}

async fn ensure_ssm_session_document(
    region: &str,
    artifacts: &mut BTreeMap<String, String>,
    event_sink: &mut dyn InstallEventSink,
) {
    match ssm_session_document_exists(region).await {
        Ok(true) => {
            artifacts.insert(
                "ssm_session_document".into(),
                SSM_SESSION_DOCUMENT_NAME.into(),
            );
            let _ = emit_sync_log(
                event_sink,
                &format!("SSM session document ready: {SSM_SESSION_DOCUMENT_NAME}"),
            );
        }
        Ok(false) => match create_ssm_session_document(region).await {
            Ok(()) => {
                artifacts.insert(
                    "ssm_session_document".into(),
                    SSM_SESSION_DOCUMENT_NAME.into(),
                );
                let _ = emit_sync_log(
                    event_sink,
                    &format!("Created SSM session document: {SSM_SESSION_DOCUMENT_NAME}"),
                );
            }
            Err(err) => {
                let _ = emit_sync_log(
                    event_sink,
                    &format!(
                        "Warning: could not create {SSM_SESSION_DOCUMENT_NAME} SSM document ({err})"
                    ),
                );
            }
        },
        Err(err) => {
            let _ = emit_sync_log(
                event_sink,
                &format!(
                    "Warning: could not verify {SSM_SESSION_DOCUMENT_NAME} SSM document ({err})"
                ),
            );
        }
    }
}

async fn ssm_session_document_exists(region: &str) -> Result<bool, AdapterError> {
    let output = run_command(&aws_ssm_command(
        region,
        &["describe-document", "--name", SSM_SESSION_DOCUMENT_NAME],
    ))
    .await?;
    Ok(output.success())
}

async fn create_ssm_session_document(region: &str) -> Result<(), AdapterError> {
    let output = run_command(&aws_ssm_command(
        region,
        &[
            "create-document",
            "--name",
            SSM_SESSION_DOCUMENT_NAME,
            "--document-type",
            "Session",
            "--content",
            r#"{"schemaVersion":"1.0","description":"SSM session for Loki - starts as ec2-user","sessionType":"Standard_Stream","inputs":{"runAsEnabled":true,"runAsDefaultUser":"ec2-user","shellProfile":{"linux":"cd ~ && exec bash --login"}}}"#,
        ],
    ))
    .await?;
    if output.success() {
        Ok(())
    } else {
        let detail = if !output.stderr.trim().is_empty() {
            output.stderr
        } else {
            output.stdout
        };
        Err(AdapterError::Message(detail))
    }
}

async fn get_ssm_parameter(region: &str, name: &str) -> Result<Option<String>, AdapterError> {
    let output = run_command(&aws_ssm_command(
        region,
        &[
            "get-parameter",
            "--name",
            name,
            "--query",
            "Parameter.Value",
            "--output",
            "text",
        ],
    ))
    .await?;
    if output.success() {
        let value = output.stdout.trim().to_string();
        return Ok((!value.is_empty()).then_some(value));
    }
    Ok(None)
}

async fn send_ssm_shell_command(
    region: &str,
    instance_id: &str,
    shell_command: &str,
) -> Result<Option<String>, AdapterError> {
    let parameters = format!(r#"commands=["{shell_command}"]"#);
    let output = run_command(&aws_ssm_command(
        region,
        &[
            "send-command",
            "--instance-ids",
            instance_id,
            "--document-name",
            "AWS-RunShellScript",
            "--parameters",
            &parameters,
            "--query",
            "Command.CommandId",
            "--output",
            "text",
        ],
    ))
    .await?;
    if output.success() {
        let value = output.stdout.trim().to_string();
        Ok((!value.is_empty()).then_some(value))
    } else {
        Ok(None)
    }
}

async fn get_ssm_command_output(
    region: &str,
    instance_id: &str,
    command_id: &str,
) -> Result<String, AdapterError> {
    let output = run_command(&aws_ssm_command(
        region,
        &[
            "get-command-invocation",
            "--command-id",
            command_id,
            "--instance-id",
            instance_id,
            "--query",
            "StandardOutputContent",
            "--output",
            "text",
        ],
    ))
    .await?;
    if output.success() {
        Ok(output.stdout)
    } else {
        Ok(String::new())
    }
}

fn aws_ssm_command(region: &str, args: &[&str]) -> CommandSpec {
    let mut full_args = args
        .iter()
        .map(|value| (*value).to_string())
        .collect::<Vec<_>>();
    full_args.extend(["--region".into(), region.into()]);
    CommandSpec {
        program: "aws".into(),
        args: full_args,
        current_dir: None,
        env: BTreeMap::new(),
    }
}

fn parse_existing_resources(stderr: &str) -> Vec<(String, String)> {
    let mut resources = Vec::new();
    let mut current_error_line: Option<&str> = None;

    for line in stderr.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Error:") {
            let is_existing_resource_error = trimmed.contains("EntityAlreadyExists")
                || trimmed.to_ascii_lowercase().contains("already exists");
            current_error_line = is_existing_resource_error.then_some(trimmed);
            continue;
        }

        let Some(error_line) = current_error_line else {
            continue;
        };

        let Some(resource_address) = parse_resource_address(trimmed) else {
            continue;
        };

        let Some(import_id) = extract_import_id(error_line, &resource_address) else {
            current_error_line = None;
            continue;
        };

        if !resources
            .iter()
            .any(|(address, id)| address == &resource_address && id == &import_id)
        {
            resources.push((resource_address, import_id));
        }
        current_error_line = None;
    }

    resources
}

fn parse_resource_address(line: &str) -> Option<String> {
    let rest = line.strip_prefix("with ")?;
    let address = rest.split(',').next()?.trim();
    (!address.is_empty()).then(|| address.to_string())
}

fn extract_import_id(error_line: &str, resource_address: &str) -> Option<String> {
    let resource_type = supported_resource_type(resource_address)?;
    match resource_type {
        "aws_iam_role" | "aws_iam_instance_profile" | "aws_s3_bucket" => {
            extract_parenthesized_value(error_line)
        }
        "aws_security_group" => extract_id_with_prefix(error_line, "sg-")
            .or_else(|| extract_parenthesized_value(error_line)),
        "aws_vpc" => extract_id_with_prefix(error_line, "vpc-")
            .or_else(|| extract_parenthesized_value(error_line)),
        _ => None,
    }
}

fn supported_resource_type(resource_address: &str) -> Option<&str> {
    resource_address
        .split('.')
        .find_map(|segment| match segment {
            "aws_iam_role" => Some("aws_iam_role"),
            "aws_iam_instance_profile" => Some("aws_iam_instance_profile"),
            "aws_s3_bucket" => Some("aws_s3_bucket"),
            "aws_security_group" => Some("aws_security_group"),
            "aws_vpc" => Some("aws_vpc"),
            _ => None,
        })
}

fn extract_parenthesized_value(line: &str) -> Option<String> {
    let start = line.find('(')?;
    let end = line[start + 1..].find(')')?;
    let value = line[start + 1..start + 1 + end].trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn extract_id_with_prefix(line: &str, prefix: &str) -> Option<String> {
    line.split(|c: char| c.is_whitespace() || matches!(c, '(' | ')' | ':' | ',' | ';' | '.'))
        .find(|token| token.starts_with(prefix))
        .map(str::to_string)
}

fn parse_terraform_outputs(raw: &str) -> Result<BTreeMap<String, String>, AdapterError> {
    let parsed: BTreeMap<String, TerraformOutputValue> =
        serde_json::from_str(raw).map_err(|source| {
            AdapterError::Message(format!("Failed to parse terraform output JSON — {source}"))
        })?;

    let mut artifacts = BTreeMap::new();
    artifacts.insert("terraform_output_json".into(), raw.to_string());

    for (key, value) in parsed {
        let text = match value.value {
            serde_json::Value::String(text) => text,
            other => other.to_string(),
        };
        match key.as_str() {
            "instance_id" | "public_ip" | "private_ip" | "vpc_id" | "security_group_id"
            | "role_arn" | "ssm_connect" | "pack_name" | "profile_name" => {
                artifacts.insert(key, text);
            }
            _ => {}
        }
    }

    Ok(artifacts)
}

fn ensure_terraform_success(
    output: &CommandOutput,
    default_message: &str,
) -> Result<(), AdapterError> {
    if output.success() {
        return Ok(());
    }

    let stderr = output.stderr.trim();
    if stderr.contains("No such file or directory") || stderr.contains("command not found") {
        return Err(AdapterError::Message(
            "Terraform CLI not found — install terraform and re-run the installer".into(),
        ));
    }

    if stderr.contains("No valid credential sources found")
        || stderr.contains("failed to refresh cached credentials")
    {
        return Err(AdapterError::Message(
            "AWS credentials not found — run aws configure or export AWS_ACCESS_KEY_ID".into(),
        ));
    }

    let mut message = default_message.to_string();
    if !stderr.is_empty() {
        message.push_str(": ");
        message.push_str(stderr);
    } else if !output.stdout.is_empty() {
        message.push_str(": ");
        message.push_str(output.stdout.trim());
    }
    Err(AdapterError::Message(message))
}

async fn record_command_artifact(
    key: &str,
    output: CommandOutput,
    artifacts: &mut BTreeMap<String, String>,
    event_sink: &mut dyn InstallEventSink,
) {
    let value = if output.stdout.is_empty() {
        output.stderr
    } else {
        output.stdout
    };
    artifacts.insert(key.into(), value.clone());
    event_sink
        .emit(InstallEvent::ArtifactRecorded {
            key: key.into(),
            value,
        })
        .await;
}

async fn emit_step_started(
    session: &mut InstallSession,
    event_sink: &mut dyn InstallEventSink,
    step: &DeployStep,
) {
    update_session_phase(session, step.phase);
    event_sink
        .emit(InstallEvent::PhaseStarted {
            phase: step.phase,
            message: step.display_name.clone(),
        })
        .await;
    event_sink
        .emit(InstallEvent::StepStarted {
            step_id: step.id.clone(),
            display_name: step.display_name.clone(),
        })
        .await;
}

async fn emit_step_finished(
    event_sink: &mut dyn InstallEventSink,
    step: &DeployStep,
    message: &str,
) {
    event_sink
        .emit(InstallEvent::StepFinished {
            step_id: step.id.clone(),
            message: message.into(),
        })
        .await;
}

fn phase_rank(phase: InstallPhase) -> u8 {
    match phase {
        InstallPhase::ValidateEnvironment => 0,
        InstallPhase::DiscoverAwsContext => 1,
        InstallPhase::ResolveMetadata => 2,
        InstallPhase::PrepareDeployment => 3,
        InstallPhase::PlanDeployment => 4,
        InstallPhase::ApplyDeployment => 5,
        InstallPhase::WaitForResources => 6,
        InstallPhase::Finalize => 7,
        InstallPhase::PostInstall => 8,
    }
}

fn phase_is_past(current: InstallPhase, target: InstallPhase) -> bool {
    phase_rank(current) > phase_rank(target)
}

fn completed_apply_result(plan: &InstallPlan, session: &InstallSession) -> ApplyResult {
    ApplyResult {
        final_phase: InstallPhase::PostInstall,
        artifacts: session.artifacts.clone(),
        post_install_steps: plan.post_install_steps.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        TerraformContext, adapter_option_to_tf_var, build_terraform_apply_command,
        build_terraform_import_command, build_terraform_init_command, build_terraform_plan_command,
        parse_df_available_mb, parse_existing_resources, parse_terraform_outputs, phase_is_past,
        terraform_init_env,
    };
    use crate::core::InstallPhase;

    fn test_context() -> TerraformContext {
        TerraformContext {
            terraform_bin: "/usr/local/bin/terraform".into(),
            working_dir: "/tmp/repo/deploy/terraform".into(),
            plan_file: "/tmp/repo/tfplan".into(),
            region: "us-east-1".into(),
            pack: "openclaw".into(),
            profile: "builder".into(),
            environment_name: "loki-openclaw".into(),
            tf_vars: Vec::new(),
        }
    }

    #[test]
    fn terraform_init_command_uses_working_dir_and_non_interactive_flags() {
        let command = build_terraform_init_command(&test_context());
        assert_eq!(command.program, "/usr/local/bin/terraform");
        assert_eq!(
            command.args,
            vec![
                "-chdir=/tmp/repo/deploy/terraform",
                "init",
                "-no-color",
                "-input=false",
            ]
        );
        assert_eq!(
            command.env.get("TF_PLUGIN_CACHE_DIR").map(String::as_str),
            Some("/tmp/terraform-plugin-cache")
        );
    }

    #[test]
    fn terraform_plan_command_includes_outfile_and_tf_vars() {
        let command = build_terraform_plan_command(
            &test_context(),
            &[
                (
                    "default_model".into(),
                    "us.anthropic.claude-opus-4-6-v1".into(),
                ),
                ("openclaw_gateway_port".into(), "3001".into()),
                ("bedrockify_port".into(), "8090".into()),
                (
                    "haiku_model".into(),
                    "us.anthropic.claude-haiku-4-5-20251001-v1:0".into(),
                ),
            ],
        );
        assert_eq!(command.program, "/usr/local/bin/terraform");
        assert!(command.args.contains(&"plan".into()));
        assert!(command.args.contains(&"-no-color".into()));
        assert!(command.args.contains(&"-input=false".into()));
        assert!(command.args.contains(&"-out=/tmp/repo/tfplan".into()));
        assert!(command.args.contains(&"-var=aws_region=us-east-1".into()));
        assert!(command.args.contains(&"-var=pack_name=openclaw".into()));
        assert!(command.args.contains(&"-var=profile_name=builder".into()));
        assert!(
            command
                .args
                .contains(&"-var=environment_name=loki-openclaw".into())
        );
        assert!(
            command
                .args
                .contains(&"-var=default_model=us.anthropic.claude-opus-4-6-v1".into())
        );
        assert!(
            command
                .args
                .contains(&"-var=openclaw_gateway_port=3001".into())
        );
        assert!(command.args.contains(&"-var=bedrockify_port=8090".into()));
        assert!(
            command
                .args
                .contains(&"-var=haiku_model=us.anthropic.claude-haiku-4-5-20251001-v1:0".into())
        );
    }

    #[test]
    fn terraform_var_mapping_covers_pack_specific_options() {
        assert_eq!(
            adapter_option_to_tf_var("bedrockify_port"),
            Some("bedrockify_port")
        );
        assert_eq!(adapter_option_to_tf_var("embed_model"), Some("embed_model"));
        assert_eq!(
            adapter_option_to_tf_var("hermes_model"),
            Some("hermes_model")
        );
        assert_eq!(adapter_option_to_tf_var("haiku_model"), Some("haiku_model"));
        assert_eq!(
            adapter_option_to_tf_var("sandbox_name"),
            Some("sandbox_name")
        );
        assert_eq!(
            adapter_option_to_tf_var("enable_satellite_services"),
            Some("enable_satellite_services")
        );
        assert_eq!(adapter_option_to_tf_var("workspace"), None);
    }

    #[test]
    fn terraform_apply_command_uses_saved_plan() {
        let command = build_terraform_apply_command(&test_context());
        assert_eq!(command.program, "/usr/local/bin/terraform");
        assert_eq!(
            command.args,
            vec![
                "-chdir=/tmp/repo/deploy/terraform",
                "apply",
                "-no-color",
                "-input=false",
                "-auto-approve",
                "/tmp/repo/tfplan",
            ]
        );
    }

    #[test]
    fn terraform_import_command_uses_working_dir_and_resource_details() {
        let command = build_terraform_import_command(
            &test_context(),
            "aws_iam_role.instance",
            "loki-instance-role",
        );
        assert_eq!(command.program, "/usr/local/bin/terraform");
        assert_eq!(
            command.args,
            vec![
                "-chdir=/tmp/repo/deploy/terraform",
                "import",
                "-no-color",
                "aws_iam_role.instance",
                "loki-instance-role",
            ]
        );
    }

    #[test]
    fn resume_skips_only_completed_terraform_phases() {
        assert!(phase_is_past(
            InstallPhase::PlanDeployment,
            InstallPhase::PrepareDeployment
        ));
        assert!(phase_is_past(
            InstallPhase::PostInstall,
            InstallPhase::ApplyDeployment
        ));
        assert!(!phase_is_past(
            InstallPhase::ApplyDeployment,
            InstallPhase::ApplyDeployment
        ));
        assert!(!phase_is_past(
            InstallPhase::ApplyDeployment,
            InstallPhase::PostInstall
        ));
    }

    #[test]
    fn parse_terraform_outputs_extracts_instance_id_and_public_ip() {
        let artifacts = parse_terraform_outputs(
            r#"{
              "instance_id": {"value": "i-123"},
              "public_ip": {"value": "1.2.3.4"},
              "role_arn": {"value": "arn:aws:iam::123456789012:role/example"}
            }"#,
        )
        .expect("parse terraform outputs");

        assert_eq!(artifacts.get("instance_id"), Some(&"i-123".into()));
        assert_eq!(artifacts.get("public_ip"), Some(&"1.2.3.4".into()));
        assert!(artifacts.get("terraform_output_json").is_some());
    }

    #[test]
    fn parse_df_available_mb_reads_available_column() {
        let output = "Filesystem 1048576-blocks Used Available Capacity Mounted on\n/dev/root 1000 450 550 45% /";
        assert_eq!(parse_df_available_mb(output), Some(550));
    }

    #[test]
    fn terraform_init_env_sets_plugin_cache_dir() {
        let env = terraform_init_env();
        assert_eq!(
            env.get("TF_PLUGIN_CACHE_DIR").map(String::as_str),
            Some("/tmp/terraform-plugin-cache")
        );
    }

    #[test]
    fn parse_existing_resources_extracts_supported_resource_pairs() {
        let parsed = parse_existing_resources(
            r#"
Error: creating IAM Role (loki-instance-role): operation error IAM: CreateRole, https response error StatusCode: 409, RequestID: abc, EntityAlreadyExists: Role with name loki-instance-role already exists.

  with aws_iam_role.instance,
  on iam.tf line 10, in resource "aws_iam_role" "instance":
  10: resource "aws_iam_role" "instance" {

Error: creating IAM Instance Profile (loki-instance-profile): operation error IAM: CreateInstanceProfile, https response error StatusCode: 409, RequestID: def, EntityAlreadyExists: Instance Profile loki-instance-profile already exists.

  with module.compute.aws_iam_instance_profile.instance,
  on iam.tf line 20, in resource "aws_iam_instance_profile" "instance":

Error: creating S3 Bucket (loki-bucket): BucketAlreadyOwnedByYou: Bucket with name loki-bucket already exists.

  with aws_s3_bucket.artifacts,
  on s3.tf line 2, in resource "aws_s3_bucket" "artifacts":

Error: creating Security Group (sg-0123456789abcdef0): InvalidGroup.Duplicate: security group already exists.

  with aws_security_group.instance,
  on network.tf line 5, in resource "aws_security_group" "instance":

Error: creating VPC (vpc-0123456789abcdef0): operation error EC2: CreateVpc, https response error StatusCode: 400, RequestID: xyz, VpcLimitExceeded: The vpc vpc-0123456789abcdef0 already exists.

  with module.network.aws_vpc.main,
  on network.tf line 1, in resource "aws_vpc" "main":
"#,
        );

        assert_eq!(
            parsed,
            vec![
                ("aws_iam_role.instance".into(), "loki-instance-role".into()),
                (
                    "module.compute.aws_iam_instance_profile.instance".into(),
                    "loki-instance-profile".into(),
                ),
                ("aws_s3_bucket.artifacts".into(), "loki-bucket".into()),
                (
                    "aws_security_group.instance".into(),
                    "sg-0123456789abcdef0".into(),
                ),
                (
                    "module.network.aws_vpc.main".into(),
                    "vpc-0123456789abcdef0".into(),
                ),
            ]
        );
    }

    #[test]
    fn parse_existing_resources_ignores_unsupported_or_unmatched_errors() {
        let parsed = parse_existing_resources(
            r#"
Error: creating Subnet (subnet-0123456789abcdef0): operation error EC2: CreateSubnet, https response error StatusCode: 400, RequestID: xyz, InvalidSubnet.Conflict: subnet already exists.

  with aws_subnet.main,
  on network.tf line 30, in resource "aws_subnet" "main":

Error: reading IAM Role (loki-instance-role): some unrelated read error.

  with aws_iam_role.instance,
  on iam.tf line 10, in resource "aws_iam_role" "instance":
"#,
        );

        assert!(parsed.is_empty());
    }
}
