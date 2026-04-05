use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstallRequest {
    pub engine: InstallerEngine,
    pub mode: InstallMode,
    pub pack: String,
    pub profile: Option<String>,
    pub method: Option<DeployMethodId>,
    pub region: Option<String>,
    pub stack_name: Option<String>,
    pub auto_yes: bool,
    pub json_output: bool,
    pub resume_session_id: Option<String>,
    pub extra_options: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InstallerEngine {
    V1,
    V2,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InstallMode {
    Interactive,
    NonInteractive,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum DeployMethodId {
    Cfn,
    Terraform,
}

impl std::fmt::Display for DeployMethodId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cfn => write!(f, "cfn"),
            Self::Terraform => write!(f, "terraform"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstallPlan {
    pub request: InstallRequest,
    pub resolved_pack: PackManifest,
    pub resolved_profile: ProfileManifest,
    pub resolved_method: MethodManifest,
    pub resolved_region: String,
    pub resolved_stack_name: Option<String>,
    pub prerequisites: Vec<PrerequisiteCheck>,
    pub deploy_steps: Vec<DeployStep>,
    pub warnings: Vec<PlanWarning>,
    pub post_install_steps: Vec<PostInstallStep>,
    pub session_persistence: SessionPersistenceSpec,
    pub adapter_options: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrerequisiteCheck {
    pub id: String,
    pub display_name: String,
    pub kind: PrerequisiteKind,
    pub required: bool,
    pub remediation: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PrerequisiteKind {
    OsSupported,
    ArchSupported,
    AwsCliPresent,
    AwsCredentialsValid,
    AwsCallerIdentityResolvable,
    NetworkReachable,
    BinaryDownloadable,
    MethodToolingPresent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeployStep {
    pub id: String,
    pub phase: InstallPhase,
    pub display_name: String,
    pub action: DeployAction,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InstallPhase {
    ValidateEnvironment,
    DiscoverAwsContext,
    ResolveMetadata,
    PrepareDeployment,
    PlanDeployment,
    ApplyDeployment,
    WaitForResources,
    Finalize,
    PostInstall,
}

impl std::fmt::Display for InstallPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::ValidateEnvironment => "validate_environment",
            Self::DiscoverAwsContext => "discover_aws_context",
            Self::ResolveMetadata => "resolve_metadata",
            Self::PrepareDeployment => "prepare_deployment",
            Self::PlanDeployment => "plan_deployment",
            Self::ApplyDeployment => "apply_deployment",
            Self::WaitForResources => "wait_for_resources",
            Self::Finalize => "finalize",
            Self::PostInstall => "post_install",
        };
        write!(f, "{label}")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DeployAction {
    RunCommand { program: String, args: Vec<String> },
    CreateStack,
    UpdateStack,
    DestroyStack,
    WaitForStack,
    VerifyInstanceHealth,
    EmitInstructions,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanWarning {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PostInstallStep {
    pub id: String,
    pub display_name: String,
    pub instruction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionPersistenceSpec {
    pub format: SessionFormat,
    pub path_hint: String,
    pub persist_phases: Vec<InstallPhase>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionFormat {
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackManifest {
    pub schema_version: u32,
    pub id: String,
    pub display_name: String,
    pub description: Option<String>,
    pub experimental: bool,
    pub allowed_profiles: Vec<String>,
    pub supported_methods: Vec<DeployMethodId>,
    pub default_profile: Option<String>,
    pub default_method: Option<DeployMethodId>,
    pub default_region: Option<String>,
    pub post_install: Vec<PostInstallActionId>,
    pub required_env: Vec<String>,
    pub extra_options_schema: BTreeMap<String, PackOptionSpec>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PostInstallActionId {
    SsmSession,
    Pairing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackOptionSpec {
    pub value_type: OptionValueType,
    pub required: bool,
    pub default_value: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OptionValueType {
    String,
    Integer,
    Boolean,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileManifest {
    pub schema_version: u32,
    pub id: String,
    pub display_name: String,
    pub description: Option<String>,
    pub supported_packs: Vec<String>,
    pub default_method: Option<DeployMethodId>,
    pub default_region: Option<String>,
    pub config: BTreeMap<String, String>,
    pub tags: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MethodManifest {
    pub schema_version: u32,
    pub id: DeployMethodId,
    pub display_name: String,
    pub description: Option<String>,
    pub requires_stack_name: bool,
    pub requires_region: bool,
    pub required_tools: Vec<String>,
    pub supports_resume: bool,
    pub supports_uninstall: bool,
    pub input_schema: BTreeMap<String, MethodOptionSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MethodOptionSpec {
    pub value_type: OptionValueType,
    pub required: bool,
    pub default_value: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdapterPlan {
    pub prerequisites: Vec<PrerequisiteCheck>,
    pub deploy_steps: Vec<DeployStep>,
    pub adapter_options: BTreeMap<String, String>,
    pub warnings: Vec<PlanWarning>,
    pub post_install_steps: Vec<PostInstallStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstallSession {
    pub session_id: String,
    pub installer_version: String,
    pub engine: InstallerEngine,
    pub mode: InstallMode,
    pub request: InstallRequest,
    pub plan: Option<InstallPlan>,
    pub phase: InstallPhase,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub artifacts: BTreeMap<String, String>,
    pub status_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApplyResult {
    pub final_phase: InstallPhase,
    pub artifacts: BTreeMap<String, String>,
    pub post_install_steps: Vec<PostInstallStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UninstallResult {
    pub removed_artifacts: BTreeMap<String, String>,
    pub warnings: Vec<PlanWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeployStatus {
    pub deployed: bool,
    pub pack: String,
    pub profile: String,
    pub method: DeployMethodId,
    pub region: Option<String>,
    pub stack_name: Option<String>,
    pub stack_status: Option<String>,
    pub instance_health: Option<String>,
    pub last_updated_at: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum AdapterValidationError {
    #[error("missing required field: {0}")]
    MissingField(&'static str),
    #[error("unsupported value for {field}: {value}")]
    UnsupportedValue { field: &'static str, value: String },
    #[error("invalid option: {0}")]
    InvalidOption(String),
}

#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("preflight failed: {0}")]
    Preflight(String),
    #[error("command failed: {program}")]
    CommandFailed { program: String, stderr: String },
    #[error("session is not resumable")]
    NotResumable,
    #[error("deployment state missing: {0}")]
    MissingArtifact(&'static str),
    #[error("{0}")]
    Other(String),
}

#[async_trait]
pub trait InstallEventSink: Send {
    async fn emit(&mut self, event: InstallEvent);
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InstallEvent {
    PhaseStarted {
        phase: InstallPhase,
        message: String,
    },
    StepStarted {
        step_id: String,
        message: String,
    },
    StepFinished {
        step_id: String,
        message: String,
    },
    Warning {
        code: String,
        message: String,
    },
    ArtifactRecorded {
        key: String,
        value: String,
    },
    LogLine {
        message: String,
    },
}

#[async_trait]
pub trait DeployAdapter: Send + Sync {
    fn method_id(&self) -> DeployMethodId;

    fn validate_request(
        &self,
        request: &InstallRequest,
        pack: &PackManifest,
        profile: Option<&ProfileManifest>,
        method: &MethodManifest,
    ) -> Result<(), AdapterValidationError>;

    async fn build_plan(
        &self,
        request: &InstallRequest,
        pack: &PackManifest,
        profile: &ProfileManifest,
        method: &MethodManifest,
    ) -> Result<AdapterPlan, AdapterError>;

    async fn apply(
        &self,
        plan: &InstallPlan,
        session: &mut InstallSession,
        event_sink: &mut dyn InstallEventSink,
    ) -> Result<ApplyResult, AdapterError>;

    async fn resume(
        &self,
        session: &mut InstallSession,
        event_sink: &mut dyn InstallEventSink,
    ) -> Result<ApplyResult, AdapterError>;

    async fn uninstall(
        &self,
        session: &InstallSession,
        event_sink: &mut dyn InstallEventSink,
    ) -> Result<UninstallResult, AdapterError>;

    async fn status(&self, session: &InstallSession) -> Result<DeployStatus, AdapterError>;
}
