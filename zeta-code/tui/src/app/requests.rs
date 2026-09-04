use super::App;
use super::AppCommand;
use super::AppEvent;
use super::completion::Completion;
use crate::client;
use std::collections::BTreeMap;

/// A backend or host resource whose operations must remain ordered.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum RequestKey {
    Interrupt,
    Interaction,
    Thread,
    Config,
    Keymap,
    StatusLine,
    Connectors,
    Directories,
    Sessions,
    Mcp,
    Skills,
    Clipboard,
    FileExport,
    Git,
}

#[derive(Default)]
pub(super) struct RequestTasks {
    tasks: BTreeMap<RequestKey, client::RequestTask<Completion>>,
}

impl RequestTasks {
    pub(super) fn is_idle(&self, key: Option<RequestKey>) -> bool {
        key.is_none_or(|key| !self.tasks.contains_key(&key))
    }

    pub(super) fn spawn(
        &mut self,
        key: Option<RequestKey>,
        name: &'static str,
        request: impl FnOnce() -> Completion + Send + 'static,
        app: &mut App,
    ) {
        let Some(key) = key else {
            app.update(AppEvent::FailureReported(format!(
                "background request `{name}` has no request key"
            )));
            return;
        };
        if self.tasks.contains_key(&key) {
            app.update(AppEvent::FailureReported(format!(
                "background request `{name}` conflicts with an active {key:?} request"
            )));
            return;
        }
        match client::RequestTask::spawn(name, request) {
            Ok(task) => {
                self.tasks.insert(key, task);
            }
            Err(error) => app.update(AppEvent::FailureReported(format!(
                "could not start background request: {error}"
            ))),
        }
    }

    pub(super) fn poll(&mut self) -> Vec<Result<Completion, std::io::Error>> {
        let mut completed = Vec::new();
        let keys = self.tasks.keys().copied().collect::<Vec<_>>();
        for key in keys {
            let result = self
                .tasks
                .get_mut(&key)
                .expect("the request key was collected from the active task map")
                .poll();
            match result {
                Ok(Some(completion)) => {
                    self.tasks.remove(&key);
                    completed.push(Ok(completion));
                }
                Ok(None) => {}
                Err(error) => {
                    self.tasks.remove(&key);
                    completed.push(Err(error));
                }
            }
        }
        completed
    }
}

pub(super) fn request_key(command: &AppCommand) -> Option<RequestKey> {
    match command {
        AppCommand::Interrupt => Some(RequestKey::Interrupt),
        AppCommand::ResolveThreadRequest(_) => Some(RequestKey::Interaction),
        AppCommand::CopyLastResponse
        | AppCommand::ReadClipboardImage
        | AppCommand::RefreshClipboardImageAvailability => Some(RequestKey::Clipboard),
        AppCommand::ExportTranscript { .. } => Some(RequestKey::FileExport),
        AppCommand::Quit | AppCommand::Suspend | AppCommand::CycleNextApprovalMode => None,
        AppCommand::OpenConfigEditor
        | AppCommand::OpenThemePicker
        | AppCommand::OpenCustomThemePicker
        | AppCommand::EditConfig(_)
        | AppCommand::SetProviderApiKey(_)
        | AppCommand::SetPreferredModel { .. }
        | AppCommand::SetCustomTheme { .. }
        | AppCommand::SetTheme { .. } => Some(RequestKey::Config),
        AppCommand::OpenKeymapEditor | AppCommand::EditKeymap(_) => Some(RequestKey::Keymap),
        AppCommand::OpenStatusLineEditor | AppCommand::EditStatusLine(_) => {
            Some(RequestKey::StatusLine)
        }
        AppCommand::ConnectConnectorDeviceOAuth { .. } | AppCommand::DisconnectConnector { .. } => {
            Some(RequestKey::Connectors)
        }
        AppCommand::RemoveDir { .. } | AppCommand::SetDirPermissions(_) => {
            Some(RequestKey::Directories)
        }
        AppCommand::ResumeSession { .. }
        | AppCommand::ArchiveSessions { .. }
        | AppCommand::CreateSessionAndEnter { .. } => Some(RequestKey::Sessions),
        AppCommand::SetMcpEnablement { .. } => Some(RequestKey::Mcp),
        AppCommand::SetSkillEnablement { .. } => Some(RequestKey::Skills),
        AppCommand::ExecuteProductCommand(_)
        | AppCommand::LoadOlderHistory
        | AppCommand::OpenRewindPicker
        | AppCommand::RewindToCheckpoint { .. }
        | AppCommand::SwitchThread { .. }
        | AppCommand::SubmitTurn { .. }
        | AppCommand::SubmitQueuedTurn { .. }
        | AppCommand::SteerTurn { .. } => Some(RequestKey::Thread),
    }
}
