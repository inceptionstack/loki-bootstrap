use crate::core::{
    ApplyResult, DeployStatus, InstallEvent, InstallPlan, InstallSession, MethodManifest,
    PackManifest, ProfileManifest,
};
use crossterm::event::KeyEvent;

#[derive(Debug)]
pub enum InstallerEvent {
    AppStarted,
    Tick,
    KeyPressed(KeyEvent),
    Resize { width: u16, height: u16 },
    PacksLoaded(Result<Vec<PackManifest>, String>),
    ProfilesLoaded(Result<Vec<ProfileManifest>, String>),
    MethodsLoaded(Result<Vec<MethodManifest>, String>),
    DoctorCompleted(Result<crate::core::DoctorReport, String>),
    PlanBuilt(Result<InstallPlan, String>),
    SessionLoaded(Result<InstallSession, String>),
    InstallEventReceived(InstallEvent),
    DeployFinished(Result<ApplyResult, String>),
    StatusLoaded(Result<DeployStatus, String>),
    BackRequested,
    NextRequested,
    QuitRequested,
    ErrorAcknowledged,
}
