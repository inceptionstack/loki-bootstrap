use crate::core::{
    AdapterError, AdapterPlan, AdapterValidationError, ApplyResult, DeployAction, DeployAdapter,
    DeployMethodId, DeployStatus, DeployStep, InstallEvent, InstallEventSink, InstallPhase,
    InstallPlan, InstallRequest, InstallSession, MethodManifest, PackManifest, PlanWarning,
    PostInstallStep, PrerequisiteCheck, PrerequisiteKind, ProfileManifest, UninstallResult,
};
use chrono::Utc;
use std::collections::BTreeMap;
use tokio::time::{Duration, sleep};

pub struct CfnAdapter;

#[async_trait::async_trait]
impl DeployAdapter for CfnAdapter {
    fn method_id(&self) -> DeployMethodId {
        DeployMethodId::Cfn
    }

    fn validate_request(
        &self,
        request: &InstallRequest,
        _pack: &PackManifest,
        _profile: Option<&ProfileManifest>,
        method: &MethodManifest,
    ) -> Result<(), AdapterValidationError> {
        if method.requires_stack_name && request.stack_name.is_none() {
            return Err(AdapterValidationError::MissingField("stack_name"));
        }
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
        adapter_options.insert(
            "template_path".into(),
            "deploy/cloudformation/template.yaml".into(),
        );
        adapter_options.insert("pack".into(), pack.id.clone());
        adapter_options.insert("profile".into(), profile.id.clone());
        if let Some(region) = &request.region {
            adapter_options.insert("region".into(), region.clone());
        }

        Ok(AdapterPlan {
            prerequisites: vec![
                PrerequisiteCheck {
                    id: "aws_cli".into(),
                    display_name: "AWS CLI available".into(),
                    kind: PrerequisiteKind::AwsCliPresent,
                    required: true,
                    remediation: Some("Install aws and re-run the installer.".into()),
                },
                PrerequisiteCheck {
                    id: "cloudformation_template".into(),
                    display_name: "CloudFormation template present".into(),
                    kind: PrerequisiteKind::BinaryDownloadable,
                    required: true,
                    remediation: Some("Ensure deploy/cloudformation/template.yaml exists.".into()),
                },
            ],
            deploy_steps: vec![
                DeployStep {
                    id: "validate-environment".into(),
                    phase: InstallPhase::ValidateEnvironment,
                    display_name: "Validate environment".into(),
                    action: DeployAction::RunCommand {
                        program: "aws".into(),
                        args: vec!["--version".into()],
                    },
                },
                DeployStep {
                    id: "create-stack".into(),
                    phase: InstallPhase::ApplyDeployment,
                    display_name: "Create or update CloudFormation stack".into(),
                    action: DeployAction::CreateStack,
                },
                DeployStep {
                    id: "wait-stack".into(),
                    phase: InstallPhase::WaitForResources,
                    display_name: "Wait for stack completion".into(),
                    action: DeployAction::WaitForStack,
                },
                DeployStep {
                    id: "emit-post-install".into(),
                    phase: InstallPhase::PostInstall,
                    display_name: "Emit post-install instructions".into(),
                    action: DeployAction::EmitInstructions,
                },
            ],
            adapter_options,
            warnings: Vec::new(),
            post_install_steps: vec![PostInstallStep {
                id: "cfn_outputs".into(),
                display_name: "Inspect stack outputs".into(),
                instruction: format!(
                    "aws cloudformation describe-stacks --stack-name {}",
                    request
                        .stack_name
                        .clone()
                        .unwrap_or_else(|| format!("loki-{}", pack.id))
                ),
            }],
        })
    }

    async fn apply(
        &self,
        plan: &InstallPlan,
        session: &mut InstallSession,
        event_sink: &mut dyn InstallEventSink,
    ) -> Result<ApplyResult, AdapterError> {
        run_stubbed_apply(plan, session, event_sink, "CREATE_COMPLETE").await
    }

    async fn resume(
        &self,
        session: &mut InstallSession,
        event_sink: &mut dyn InstallEventSink,
    ) -> Result<ApplyResult, AdapterError> {
        let plan = session.plan.clone().ok_or(AdapterError::NotResumable)?;
        run_stubbed_apply(&plan, session, event_sink, "CREATE_COMPLETE").await
    }

    async fn uninstall(
        &self,
        session: &InstallSession,
        event_sink: &mut dyn InstallEventSink,
    ) -> Result<UninstallResult, AdapterError> {
        event_sink
            .emit(InstallEvent::LogLine {
                message: format!("would delete stack {:?}", session.request.stack_name),
            })
            .await;
        Ok(UninstallResult {
            removed_artifacts: BTreeMap::from([(
                "stack_name".into(),
                session.request.stack_name.clone().unwrap_or_default(),
            )]),
            warnings: vec![PlanWarning {
                code: "stubbed_uninstall".into(),
                message: "CloudFormation uninstall is currently a stub.".into(),
            }],
        })
    }

    async fn status(&self, session: &InstallSession) -> Result<DeployStatus, AdapterError> {
        Ok(DeployStatus {
            deployed: session.phase == InstallPhase::PostInstall,
            pack: session.request.pack.clone(),
            profile: session.request.profile.clone().unwrap_or_default(),
            method: DeployMethodId::Cfn,
            region: session.request.region.clone(),
            stack_name: session.request.stack_name.clone(),
            stack_status: Some(
                session
                    .artifacts
                    .get("stack_status")
                    .cloned()
                    .unwrap_or_else(|| "UNKNOWN".into()),
            ),
            instance_health: session.artifacts.get("instance_health").cloned(),
            last_updated_at: session.updated_at,
        })
    }
}

async fn run_stubbed_apply(
    plan: &InstallPlan,
    session: &mut InstallSession,
    event_sink: &mut dyn InstallEventSink,
    stack_status: &str,
) -> Result<ApplyResult, AdapterError> {
    let mut artifacts = BTreeMap::new();
    for step in &plan.deploy_steps {
        session.phase = step.phase;
        session.updated_at = Utc::now();
        event_sink
            .emit(InstallEvent::PhaseStarted {
                phase: step.phase,
                message: step.display_name.clone(),
            })
            .await;
        event_sink
            .emit(InstallEvent::StepStarted {
                step_id: step.id.clone(),
                message: step.display_name.clone(),
            })
            .await;
        sleep(Duration::from_millis(50)).await;
        event_sink
            .emit(InstallEvent::StepFinished {
                step_id: step.id.clone(),
                message: "completed".into(),
            })
            .await;
    }

    artifacts.insert(
        "stack_name".into(),
        plan.resolved_stack_name
            .clone()
            .unwrap_or_else(|| format!("loki-{}", plan.resolved_pack.id)),
    );
    artifacts.insert("stack_status".into(), stack_status.into());
    artifacts.insert("instance_health".into(), "healthy".into());

    Ok(ApplyResult {
        final_phase: InstallPhase::PostInstall,
        artifacts,
        post_install_steps: plan.post_install_steps.clone(),
    })
}
