//! Internal TUI event messages.

use crate::core::{
    InstallPhase, InstallPlan, InstallSession, MethodManifest, PackManifest, ProfileManifest,
};
use crossterm::event::KeyEvent;

#[derive(Debug)]
pub enum InstallerEvent {
    AppStarted,
    KeyPressed(KeyEvent),
    Resize {
        width: u16,
        height: u16,
    },
    PacksLoaded(Result<Vec<PackManifest>, String>),
    ProfilesLoaded(Result<Vec<ProfileManifest>, String>),
    MethodsLoaded(Result<Vec<MethodManifest>, String>),
    DoctorCompleted(Result<crate::core::DoctorReport, String>),
    PlanBuilt(Box<Result<InstallPlan, String>>),
    DeployPhaseStarted {
        phase: InstallPhase,
        message: String,
    },
    DeployStepStarted {
        step_id: String,
        display_name: String,
    },
    DeployStepFinished {
        step_id: String,
        message: String,
    },
    DeployLogLine {
        message: String,
        phase: Option<InstallPhase>,
    },
    DeployFinished(Box<InstallSession>),
    DeployFailed(String),
    Tick,
}
