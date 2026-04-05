//! Pure TUI state transition logic.

use crate::core::DeployMethodId;
use crate::tui::app::{AppLifecycle, AppState, ScreenId, TuiInstallMode, UserFacingError};
use crate::tui::events::InstallerEvent;
use crossterm::event::{KeyCode, KeyModifiers};
use std::time::Instant;

#[derive(Debug)]
pub enum AppAction {
    Render,
    LoadPacks,
    LoadProfiles { pack_id: String },
    LoadMethods { pack_id: String },
    RunDoctor,
    BuildPlan,
    StartDeploy,
    Exit,
}

pub fn update(state: &mut AppState, event: InstallerEvent) -> Vec<AppAction> {
    match event {
        InstallerEvent::AppStarted => vec![AppAction::Render],
        InstallerEvent::KeyPressed(key) => {
            if state.ui.help_visible {
                return match key.code {
                    KeyCode::Esc | KeyCode::Char('?') => {
                        state.ui.help_visible = false;
                        vec![AppAction::Render]
                    }
                    KeyCode::Char('q') => {
                        state.lifecycle = AppLifecycle::Exiting;
                        vec![AppAction::Exit]
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        state.lifecycle = AppLifecycle::Exiting;
                        vec![AppAction::Exit]
                    }
                    _ => vec![AppAction::Render],
                };
            }

            match key.code {
                KeyCode::Char('s') | KeyCode::Char('S') if state.screen == ScreenId::Welcome => {
                    state.install_mode = TuiInstallMode::Simple;
                    vec![AppAction::RunDoctor]
                }
                KeyCode::Char('a') | KeyCode::Char('A') if state.screen == ScreenId::Welcome => {
                    state.install_mode = TuiInstallMode::Advanced;
                    vec![AppAction::RunDoctor]
                }
                KeyCode::Char('a') | KeyCode::Char('A')
                    if state.screen == ScreenId::Review
                        && state.install_mode == TuiInstallMode::Simple =>
                {
                    state.install_mode = TuiInstallMode::Advanced;
                    state.screen = ScreenId::PackSelection;
                    vec![AppAction::LoadPacks]
                }
                KeyCode::Char('q') => {
                    state.lifecycle = AppLifecycle::Exiting;
                    vec![AppAction::Exit]
                }
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    state.lifecycle = AppLifecycle::Exiting;
                    vec![AppAction::Exit]
                }
                KeyCode::Char('?') => {
                    state.ui.help_visible = true;
                    vec![AppAction::Render]
                }
                KeyCode::Esc => vec![AppAction::Render],
                KeyCode::Enter | KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                    advance(state)
                }
                KeyCode::BackTab | KeyCode::Char('b') | KeyCode::Char('h') | KeyCode::Left => {
                    go_back(state)
                }
                KeyCode::Char(' ') => select_current(state),
                KeyCode::Char('r') => retry(state),
                KeyCode::Down | KeyCode::Char('j') => {
                    if state.screen == ScreenId::DeployProgress {
                        scroll_logs(state, 1);
                    } else {
                        move_cursor(state, 1);
                    }
                    vec![AppAction::Render]
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if state.screen == ScreenId::DeployProgress {
                        scroll_logs(state, -1);
                    } else {
                        move_cursor(state, -1);
                    }
                    vec![AppAction::Render]
                }
                _ => vec![AppAction::Render],
            }
        }
        InstallerEvent::PacksLoaded(result) => {
            if let Ok(packs) = result {
                state.auto_selected_pack = false;
                state.auto_selected_profile = false;
                state.auto_selected_method = false;
                state.packs = packs
                    .into_iter()
                    .filter(|pack| !pack.experimental)
                    .collect();
                state.request_draft.pack_cursor = 0;
                state.request_draft.profile_cursor = 0;
                state.request_draft.method_cursor = 0;
                if let Some(pack) = state.packs.get(state.request_draft.pack_cursor) {
                    state.request_draft.pack_id = Some(pack.id.clone());
                }
                state.screen = ScreenId::PackSelection;
                if let Some(pack_idx) = find_pack_index(&state.packs, "openclaw") {
                    state.request_draft.pack_cursor = pack_idx;
                    if let Some(pack) = state.packs.get(pack_idx) {
                        state.request_draft.pack_id = Some(pack.id.clone());
                        state.request_draft.profile_id = pack.default_profile.clone();
                        state.request_draft.method_id = pack.default_method;
                        state.auto_selected_pack = true;
                        return vec![AppAction::LoadProfiles {
                            pack_id: pack.id.clone(),
                        }];
                    }
                }
            }
            vec![AppAction::Render]
        }
        InstallerEvent::ProfilesLoaded(result) => {
            if let Ok(profiles) = result {
                state.profiles = profiles;
                state.auto_selected_profile = false;
                state.auto_selected_method = false;
                state.request_draft.profile_cursor = 0;
                if let Some(profile) = state.profiles.get(state.request_draft.profile_cursor) {
                    state.request_draft.profile_id = Some(profile.id.clone());
                }
                state.screen = ScreenId::ProfileSelection;
                if let Some(profile_idx) = find_profile_index(&state.profiles, "builder") {
                    state.request_draft.profile_cursor = profile_idx;
                    if let Some(profile) = state.profiles.get(profile_idx) {
                        state.request_draft.profile_id = Some(profile.id.clone());
                        state.auto_selected_profile = true;
                        if let Some(pack_id) = &state.request_draft.pack_id {
                            return vec![AppAction::LoadMethods {
                                pack_id: pack_id.clone(),
                            }];
                        }
                    }
                }
            }
            vec![AppAction::Render]
        }
        InstallerEvent::MethodsLoaded(result) => {
            if let Ok(methods) = result {
                state.methods = methods;
                state.auto_selected_method = false;
                state.request_draft.method_cursor = 0;
                if let Some(method) = state.methods.get(state.request_draft.method_cursor) {
                    state.request_draft.method_id = Some(method.id);
                }
                state.screen = ScreenId::MethodSelection;
                if let Some(method_idx) =
                    find_method_index(&state.methods, DeployMethodId::Terraform)
                {
                    state.request_draft.method_cursor = method_idx;
                    if let Some(method) = state.methods.get(method_idx) {
                        state.request_draft.method_id = Some(method.id);
                        state.auto_selected_method = true;
                        return vec![AppAction::BuildPlan];
                    }
                }
            }
            vec![AppAction::Render]
        }
        InstallerEvent::DoctorCompleted(result) => {
            match result {
                Ok(report) => {
                    state.doctor.report = Some(report);
                    state.screen = ScreenId::DoctorPreflight;
                }
                Err(err) => state.errors.push(UserFacingError { message: err }),
            }
            vec![AppAction::Render]
        }
        InstallerEvent::PlanBuilt(result) => {
            match *result {
                Ok(plan) => {
                    state.request_draft.region = Some(plan.resolved_region.clone());
                    state.request_draft.stack_name = plan.resolved_stack_name.clone();
                    state.plan = Some(plan);
                    state.screen = ScreenId::Review;
                }
                Err(err) => state.errors.push(UserFacingError { message: err }),
            }
            vec![AppAction::Render]
        }
        InstallerEvent::DeployPhaseStarted { phase, message } => {
            state.deployment.current_phase = Some(phase);
            state.deployment.logs.push(message);
            vec![AppAction::Render]
        }
        InstallerEvent::DeployStepStarted {
            step_id,
            display_name,
        } => {
            state.deployment.current_step_id = Some(step_id.clone());
            state.deployment.failed_steps.remove(&step_id);
            state
                .deployment
                .logs
                .push(format!("Starting {display_name} ({step_id})"));
            vec![AppAction::Render]
        }
        InstallerEvent::DeployStepFinished { step_id, message } => {
            state.deployment.completed_steps.insert(step_id.clone());
            if state.deployment.current_step_id.as_deref() == Some(step_id.as_str()) {
                state.deployment.current_step_id = None;
            }
            let summary = if message.is_empty() {
                format!("Finished {step_id}")
            } else {
                format!("Finished {step_id}: {message}")
            };
            state.deployment.logs.push(summary);
            vec![AppAction::Render]
        }
        InstallerEvent::DeployLogLine { message, phase } => {
            if let Some(phase) = phase {
                state.deployment.current_phase = Some(phase);
            }
            state.deployment.logs.push(message);
            vec![AppAction::Render]
        }
        InstallerEvent::DeployFinished(session) => {
            state.session = Some(*session);
            state.deployment.current_step_id = None;
            state.deployment.finished_at = Some(Instant::now());
            state.screen = ScreenId::PostInstall;
            vec![AppAction::Render]
        }
        InstallerEvent::DeployFailed(err) => {
            if let Some(step_id) = state.deployment.current_step_id.clone() {
                state.deployment.failed_steps.insert(step_id);
                state.deployment.current_step_id = None;
            }
            state
                .deployment
                .logs
                .push(format!("Deployment failed: {err}"));
            state.errors.push(UserFacingError { message: err });
            state.screen = ScreenId::DeployProgress;
            vec![AppAction::Render]
        }
        InstallerEvent::Tick => {
            if state.screen == ScreenId::DeployProgress {
                state.deployment.spinner_frame = (state.deployment.spinner_frame + 1)
                    % crate::tui::screens::deploy::SPINNER.len();
                state.deployment.last_tick = Some(Instant::now());
                return vec![AppAction::Render];
            }
            Vec::new()
        }
        InstallerEvent::Resize { width, height } => {
            state.ui.width = width;
            state.ui.height = height;
            vec![AppAction::Render]
        }
    }
}

fn advance(state: &mut AppState) -> Vec<AppAction> {
    match state.screen {
        ScreenId::Welcome => {
            state.install_mode = TuiInstallMode::Simple;
            vec![AppAction::RunDoctor]
        }
        ScreenId::DoctorPreflight => {
            if state.install_mode == TuiInstallMode::Simple {
                state.request_draft.pack_id = Some("openclaw".into());
                state.request_draft.profile_id = Some("builder".into());
                state.request_draft.method_id = Some(DeployMethodId::Terraform);
                state.request_draft.region = Some("us-east-1".into());
            }
            vec![AppAction::LoadPacks]
        }
        ScreenId::PackSelection => {
            if !state.auto_selected_pack
                && let Some(pack_idx) = find_pack_index(&state.packs, "openclaw")
            {
                state.request_draft.pack_cursor = pack_idx;
                state.auto_selected_pack = true;
            }
            if let Some(pack) = state.packs.get(state.request_draft.pack_cursor) {
                state.request_draft.pack_id = Some(pack.id.clone());
                state.request_draft.profile_id = pack.default_profile.clone();
                state.request_draft.method_id = pack.default_method;
                vec![AppAction::LoadProfiles {
                    pack_id: pack.id.clone(),
                }]
            } else {
                vec![AppAction::Render]
            }
        }
        ScreenId::ProfileSelection => {
            if state.install_mode == TuiInstallMode::Simple {
                state.request_draft.method_id = Some(DeployMethodId::Terraform);
                state.request_draft.region = Some("us-east-1".into());
                state.auto_selected_method = true;
                return vec![AppAction::BuildPlan];
            }
            if !state.auto_selected_profile
                && let Some(profile_idx) = find_profile_index(&state.profiles, "builder")
            {
                state.request_draft.profile_cursor = profile_idx;
                state.auto_selected_profile = true;
            }
            if let Some(profile) = state.profiles.get(state.request_draft.profile_cursor) {
                state.request_draft.profile_id = Some(profile.id.clone());
            }
            if let Some(pack_id) = &state.request_draft.pack_id {
                vec![AppAction::LoadMethods {
                    pack_id: pack_id.clone(),
                }]
            } else {
                vec![AppAction::Render]
            }
        }
        ScreenId::MethodSelection => {
            if let Some(method) = state.methods.get(state.request_draft.method_cursor) {
                state.request_draft.method_id = Some(method.id);
            }
            vec![AppAction::BuildPlan]
        }
        ScreenId::Review => {
            state.screen = ScreenId::DeployProgress;
            vec![AppAction::StartDeploy]
        }
        ScreenId::PostInstall => vec![AppAction::Render],
        ScreenId::DeployProgress => {
            if state.deployment.finished_at.is_some() {
                state.screen = ScreenId::PostInstall;
            }
            vec![AppAction::Render]
        }
    }
}

fn go_back(state: &mut AppState) -> Vec<AppAction> {
    state.screen = match state.screen {
        ScreenId::Welcome => ScreenId::Welcome,
        ScreenId::DoctorPreflight => ScreenId::Welcome,
        ScreenId::PackSelection => ScreenId::DoctorPreflight,
        ScreenId::ProfileSelection => ScreenId::PackSelection,
        ScreenId::MethodSelection => ScreenId::ProfileSelection,
        ScreenId::Review => {
            if state.install_mode == TuiInstallMode::Simple {
                ScreenId::DoctorPreflight
            } else {
                ScreenId::MethodSelection
            }
        }
        ScreenId::DeployProgress => ScreenId::DeployProgress,
        ScreenId::PostInstall => ScreenId::DeployProgress,
    };
    vec![AppAction::Render]
}

fn move_cursor(state: &mut AppState, delta: isize) {
    fn advance_index(current: usize, len: usize, delta: isize) -> usize {
        if len == 0 {
            return 0;
        }
        if delta.is_negative() {
            current.saturating_sub(delta.unsigned_abs())
        } else {
            (current + delta as usize).min(len.saturating_sub(1))
        }
    }

    match state.screen {
        ScreenId::PackSelection => {
            state.request_draft.pack_cursor =
                advance_index(state.request_draft.pack_cursor, state.packs.len(), delta);
        }
        ScreenId::ProfileSelection => {
            state.request_draft.profile_cursor = advance_index(
                state.request_draft.profile_cursor,
                state.profiles.len(),
                delta,
            );
        }
        ScreenId::MethodSelection => {
            state.request_draft.method_cursor = advance_index(
                state.request_draft.method_cursor,
                state.methods.len(),
                delta,
            );
        }
        _ => {}
    }
}

fn scroll_logs(state: &mut AppState, delta: isize) {
    let visible_lines = 30usize;
    let max_offset = state.deployment.logs.len().saturating_sub(visible_lines);
    if delta.is_negative() {
        state.deployment.scroll_offset = state
            .deployment
            .scroll_offset
            .saturating_add(delta.unsigned_abs())
            .min(max_offset);
    } else {
        state.deployment.scroll_offset = state
            .deployment
            .scroll_offset
            .saturating_sub(delta as usize);
    }
}

fn find_pack_index(packs: &[crate::core::PackManifest], target_pack_id: &str) -> Option<usize> {
    packs.iter().position(|pack| pack.id == target_pack_id)
}

fn find_profile_index(
    profiles: &[crate::core::ProfileManifest],
    target_profile_id: &str,
) -> Option<usize> {
    profiles
        .iter()
        .position(|profile| profile.id == target_profile_id)
}

fn find_method_index(
    methods: &[crate::core::MethodManifest],
    target_method_id: DeployMethodId,
) -> Option<usize> {
    methods
        .iter()
        .position(|method| method.id == target_method_id)
}

fn select_current(state: &mut AppState) -> Vec<AppAction> {
    match state.screen {
        ScreenId::PackSelection | ScreenId::ProfileSelection | ScreenId::MethodSelection => {
            advance(state)
        }
        _ => vec![AppAction::Render],
    }
}

fn retry(state: &mut AppState) -> Vec<AppAction> {
    match state.screen {
        ScreenId::Welcome | ScreenId::DoctorPreflight => vec![AppAction::RunDoctor],
        ScreenId::PackSelection => vec![AppAction::LoadPacks],
        ScreenId::ProfileSelection => state
            .request_draft
            .pack_id
            .clone()
            .map(|pack_id| vec![AppAction::LoadProfiles { pack_id }])
            .unwrap_or_else(|| vec![AppAction::Render]),
        ScreenId::MethodSelection => state
            .request_draft
            .pack_id
            .clone()
            .map(|pack_id| vec![AppAction::LoadMethods { pack_id }])
            .unwrap_or_else(|| vec![AppAction::Render]),
        ScreenId::Review => vec![AppAction::BuildPlan],
        ScreenId::DeployProgress => {
            if state.plan.is_some() {
                vec![AppAction::StartDeploy]
            } else {
                vec![AppAction::Render]
            }
        }
        ScreenId::PostInstall => vec![AppAction::Render],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        DeployMethodId, InstallMode, InstallPhase, InstallRequest, InstallSession, InstallerEngine,
        MethodManifest, PackManifest, ProfileManifest,
    };
    use chrono::Utc;
    use crossterm::event::{KeyEvent, KeyEventKind, KeyEventState};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn question_mark_toggles_help_and_escape_closes_it() {
        let mut state = AppState::default();

        let actions = update(
            &mut state,
            InstallerEvent::KeyPressed(key(KeyCode::Char('?'))),
        );
        assert!(state.ui.help_visible);
        assert!(matches!(actions.as_slice(), [AppAction::Render]));

        let actions = update(&mut state, InstallerEvent::KeyPressed(key(KeyCode::Esc)));
        assert!(!state.ui.help_visible);
        assert!(matches!(actions.as_slice(), [AppAction::Render]));
    }

    #[test]
    fn vim_and_tab_aliases_follow_existing_navigation() {
        let mut state = AppState::default();
        state.screen = ScreenId::PackSelection;
        state.packs = vec![PackManifest {
            schema_version: 1,
            id: "openclaw".into(),
            display_name: "OpenClaw".into(),
            description: None,
            experimental: false,
            allowed_profiles: vec!["builder".into()],
            supported_methods: vec![DeployMethodId::Cfn],
            default_profile: Some("builder".into()),
            default_method: Some(DeployMethodId::Cfn),
            default_region: Some("us-east-1".into()),
            post_install: vec![],
            required_env: vec![],
            extra_options_schema: Default::default(),
        }];

        let actions = update(&mut state, InstallerEvent::KeyPressed(key(KeyCode::Tab)));
        assert!(matches!(
            actions.as_slice(),
            [AppAction::LoadProfiles { pack_id }] if pack_id == "openclaw"
        ));

        state.screen = ScreenId::ProfileSelection;
        let actions = update(
            &mut state,
            InstallerEvent::KeyPressed(key(KeyCode::Char('h'))),
        );
        assert_eq!(state.screen, ScreenId::PackSelection);
        assert!(matches!(actions.as_slice(), [AppAction::Render]));
    }

    #[test]
    fn retry_replays_the_current_screen_action() {
        let mut state = AppState::default();
        state.screen = ScreenId::MethodSelection;
        state.request_draft.pack_id = Some("openclaw".into());
        state.profiles = vec![ProfileManifest {
            schema_version: 1,
            id: "builder".into(),
            display_name: "Builder".into(),
            description: None,
            supported_packs: vec!["openclaw".into()],
            default_method: Some(DeployMethodId::Cfn),
            default_region: Some("us-east-1".into()),
            config: Default::default(),
            tags: Default::default(),
        }];
        state.methods = vec![MethodManifest {
            schema_version: 1,
            id: DeployMethodId::Cfn,
            display_name: "CloudFormation".into(),
            description: None,
            requires_stack_name: true,
            requires_region: true,
            required_tools: vec!["aws".into()],
            supports_resume: true,
            supports_uninstall: true,
            input_schema: Default::default(),
        }];

        let actions = update(
            &mut state,
            InstallerEvent::KeyPressed(key(KeyCode::Char('r'))),
        );
        assert!(matches!(
            actions.as_slice(),
            [AppAction::LoadMethods { pack_id }] if pack_id == "openclaw"
        ));
    }

    #[test]
    fn packs_loaded_auto_selects_openclaw_and_filters_experimental() {
        let mut state = AppState::default();

        let actions = update(
            &mut state,
            InstallerEvent::PacksLoaded(Ok(vec![
                PackManifest {
                    schema_version: 1,
                    id: "hermes".into(),
                    display_name: "Hermes".into(),
                    description: None,
                    experimental: true,
                    allowed_profiles: vec!["builder".into()],
                    supported_methods: vec![DeployMethodId::Cfn],
                    default_profile: Some("builder".into()),
                    default_method: Some(DeployMethodId::Cfn),
                    default_region: Some("us-east-1".into()),
                    post_install: vec![],
                    required_env: vec![],
                    extra_options_schema: Default::default(),
                },
                PackManifest {
                    schema_version: 1,
                    id: "openclaw".into(),
                    display_name: "OpenClaw".into(),
                    description: None,
                    experimental: false,
                    allowed_profiles: vec!["builder".into()],
                    supported_methods: vec![DeployMethodId::Cfn],
                    default_profile: Some("builder".into()),
                    default_method: Some(DeployMethodId::Cfn),
                    default_region: Some("us-east-1".into()),
                    post_install: vec![],
                    required_env: vec![],
                    extra_options_schema: Default::default(),
                },
            ])),
        );

        assert_eq!(state.packs.len(), 1);
        assert_eq!(state.request_draft.pack_id.as_deref(), Some("openclaw"));
        assert!(state.auto_selected_pack);
        assert!(matches!(
            actions.as_slice(),
            [AppAction::LoadProfiles { pack_id }] if pack_id == "openclaw"
        ));
    }

    #[test]
    fn profiles_loaded_auto_selects_builder() {
        let mut state = AppState::default();
        state.request_draft.pack_id = Some("openclaw".into());

        let actions = update(
            &mut state,
            InstallerEvent::ProfilesLoaded(Ok(vec![
                ProfileManifest {
                    schema_version: 1,
                    id: "account_assistant".into(),
                    display_name: "Account Assistant".into(),
                    description: None,
                    supported_packs: vec!["openclaw".into()],
                    default_method: Some(DeployMethodId::Cfn),
                    default_region: Some("us-east-1".into()),
                    config: Default::default(),
                    tags: Default::default(),
                },
                ProfileManifest {
                    schema_version: 1,
                    id: "builder".into(),
                    display_name: "Builder".into(),
                    description: None,
                    supported_packs: vec!["openclaw".into()],
                    default_method: Some(DeployMethodId::Cfn),
                    default_region: Some("us-east-1".into()),
                    config: Default::default(),
                    tags: Default::default(),
                },
            ])),
        );

        assert_eq!(state.request_draft.profile_id.as_deref(), Some("builder"));
        assert!(state.auto_selected_profile);
        assert!(matches!(
            actions.as_slice(),
            [AppAction::LoadMethods { pack_id }] if pack_id == "openclaw"
        ));
    }

    #[test]
    fn methods_loaded_auto_selects_terraform_and_builds_plan() {
        let mut state = AppState::default();

        let actions = update(
            &mut state,
            InstallerEvent::MethodsLoaded(Ok(vec![
                MethodManifest {
                    schema_version: 1,
                    id: DeployMethodId::Cfn,
                    display_name: "CloudFormation".into(),
                    description: None,
                    requires_stack_name: true,
                    requires_region: true,
                    required_tools: vec!["aws".into()],
                    supports_resume: true,
                    supports_uninstall: true,
                    input_schema: Default::default(),
                },
                MethodManifest {
                    schema_version: 1,
                    id: DeployMethodId::Terraform,
                    display_name: "Terraform".into(),
                    description: None,
                    requires_stack_name: true,
                    requires_region: true,
                    required_tools: vec!["terraform".into()],
                    supports_resume: true,
                    supports_uninstall: false,
                    input_schema: Default::default(),
                },
            ])),
        );

        assert_eq!(
            state.request_draft.method_id,
            Some(DeployMethodId::Terraform)
        );
        assert_eq!(state.request_draft.method_cursor, 1);
        assert!(state.auto_selected_method);
        assert!(matches!(actions.as_slice(), [AppAction::BuildPlan]));
    }

    #[test]
    fn simple_mode_preselects_defaults_and_loads_packs_after_doctor() {
        let mut state = AppState::default();
        state.screen = ScreenId::DoctorPreflight;
        state.install_mode = TuiInstallMode::Simple;

        let actions = update(&mut state, InstallerEvent::KeyPressed(key(KeyCode::Enter)));

        assert_eq!(state.request_draft.pack_id.as_deref(), Some("openclaw"));
        assert_eq!(state.request_draft.profile_id.as_deref(), Some("builder"));
        assert_eq!(
            state.request_draft.method_id,
            Some(DeployMethodId::Terraform)
        );
        assert_eq!(state.request_draft.region.as_deref(), Some("us-east-1"));
        assert!(matches!(actions.as_slice(), [AppAction::LoadPacks]));
    }

    #[test]
    fn review_a_switches_simple_mode_to_advanced_and_reloads_packs() {
        let mut state = AppState::default();
        state.screen = ScreenId::Review;
        state.install_mode = TuiInstallMode::Simple;

        let actions = update(
            &mut state,
            InstallerEvent::KeyPressed(key(KeyCode::Char('a'))),
        );

        assert_eq!(state.install_mode, TuiInstallMode::Advanced);
        assert_eq!(state.screen, ScreenId::PackSelection);
        assert!(matches!(actions.as_slice(), [AppAction::LoadPacks]));
    }

    #[test]
    fn deploy_events_update_progress_and_terminal_state() {
        let mut state = AppState::default();
        state.screen = ScreenId::DeployProgress;

        let actions = update(
            &mut state,
            InstallerEvent::DeployStepStarted {
                step_id: "apply".into(),
                display_name: "Apply deployment".into(),
            },
        );
        assert_eq!(state.deployment.current_step_id.as_deref(), Some("apply"));
        assert!(matches!(actions.as_slice(), [AppAction::Render]));

        let actions = update(
            &mut state,
            InstallerEvent::DeployLogLine {
                message: "Planning stack".into(),
                phase: Some(InstallPhase::PlanDeployment),
            },
        );
        assert_eq!(
            state.deployment.current_phase,
            Some(InstallPhase::PlanDeployment)
        );
        assert_eq!(
            state.deployment.logs,
            vec!["Starting Apply deployment (apply)", "Planning stack"]
        );
        assert!(matches!(actions.as_slice(), [AppAction::Render]));

        let session = InstallSession {
            session_id: "session-123".into(),
            installer_version: "test".into(),
            engine: InstallerEngine::V2,
            mode: InstallMode::Interactive,
            request: InstallRequest {
                engine: InstallerEngine::V2,
                mode: InstallMode::Interactive,
                pack: "openclaw".into(),
                profile: Some("builder".into()),
                method: Some(DeployMethodId::Cfn),
                region: Some("us-east-1".into()),
                stack_name: Some("loki-openclaw".into()),
                auto_yes: true,
                json_output: false,
                resume_session_id: None,
                extra_options: Default::default(),
            },
            plan: None,
            phase: InstallPhase::PostInstall,
            started_at: Utc::now(),
            updated_at: Utc::now(),
            artifacts: Default::default(),
            status_summary: Some("deployment completed".into()),
        };

        let actions = update(
            &mut state,
            InstallerEvent::DeployFinished(Box::new(session.clone())),
        );
        assert_eq!(state.screen, ScreenId::PostInstall);
        assert_eq!(state.session.as_ref(), Some(&session));
        assert!(state.deployment.finished_at.is_some());
        assert!(matches!(actions.as_slice(), [AppAction::Render]));

        let actions = update(&mut state, InstallerEvent::DeployFailed("boom".into()));
        assert_eq!(state.screen, ScreenId::DeployProgress);
        assert_eq!(
            state.errors.last().map(|err| err.message.as_str()),
            Some("boom")
        );
        assert!(matches!(actions.as_slice(), [AppAction::Render]));
    }

    #[test]
    fn right_arrow_does_not_exit_post_install() {
        let mut state = AppState::default();
        state.screen = ScreenId::PostInstall;

        let actions = update(&mut state, InstallerEvent::KeyPressed(key(KeyCode::Right)));

        assert_eq!(state.screen, ScreenId::PostInstall);
        assert!(matches!(actions.as_slice(), [AppAction::Render]));
    }

    #[test]
    fn right_arrow_advances_deploy_progress_when_finished() {
        let mut state = AppState::default();
        state.screen = ScreenId::DeployProgress;
        state.deployment.finished_at = Some(Instant::now());

        let actions = update(&mut state, InstallerEvent::KeyPressed(key(KeyCode::Right)));

        assert_eq!(state.screen, ScreenId::PostInstall);
        assert!(matches!(actions.as_slice(), [AppAction::Render]));
    }
}
