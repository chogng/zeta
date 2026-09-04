use super::App;
use super::AppCommand;
use super::completion::Completion;
use crate::client;
use crate::config::Command as ConfigCommand;
use crate::connectors::Command as ConnectorCommand;
use crate::dirs::Command as DirCommand;
use crate::host::Command as HostCommand;
use crate::keymap::Command as KeymapCommand;
use crate::sessions::Command as SessionCommand;
use crate::status::Command as StatusCommand;
use crate::theme::Command as ThemeCommand;
use crate::thread::Command as ThreadCommand;
use crate::thread::Event as ThreadEvent;
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
            app.update(ThreadEvent::FailureReported(format!(
                "background request `{name}` has no request key"
            )));
            return;
        };
        if self.tasks.contains_key(&key) {
            app.update(ThreadEvent::FailureReported(format!(
                "background request `{name}` conflicts with an active {key:?} request"
            )));
            return;
        }
        match client::RequestTask::spawn(name, request) {
            Ok(task) => {
                self.tasks.insert(key, task);
            }
            Err(error) => app.update(ThreadEvent::FailureReported(format!(
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
        AppCommand::Thread(ThreadCommand::Interrupt) => Some(RequestKey::Interrupt),
        AppCommand::Thread(ThreadCommand::ResolveRequest(_)) => Some(RequestKey::Interaction),
        AppCommand::Host(
            HostCommand::CopyLastResponse
            | HostCommand::ReadClipboardImage
            | HostCommand::RefreshClipboardImageAvailability,
        ) => Some(RequestKey::Clipboard),
        AppCommand::Host(HostCommand::ExportTranscript { .. }) => Some(RequestKey::FileExport),
        AppCommand::Quit
        | AppCommand::Suspend
        | AppCommand::Thread(ThreadCommand::CycleNextApprovalMode) => None,
        AppCommand::Config(
            ConfigCommand::OpenEditor
            | ConfigCommand::Edit(_)
            | ConfigCommand::SetLanguageServerMode(_)
            | ConfigCommand::SetProviderApiKey(_),
        )
        | AppCommand::Theme(
            ThemeCommand::OpenPicker
            | ThemeCommand::OpenCustomPicker
            | ThemeCommand::SetCustom { .. }
            | ThemeCommand::Set { .. },
        )
        | AppCommand::Models(_) => Some(RequestKey::Config),
        AppCommand::Keymap(KeymapCommand::OpenEditor | KeymapCommand::Edit(_)) => {
            Some(RequestKey::Keymap)
        }
        AppCommand::Status(StatusCommand::OpenLineEditor | StatusCommand::EditLine(_)) => {
            Some(RequestKey::StatusLine)
        }
        AppCommand::Connectors(
            ConnectorCommand::ConnectDeviceOAuth { .. } | ConnectorCommand::Disconnect { .. },
        ) => Some(RequestKey::Connectors),
        AppCommand::Dirs(DirCommand::Remove { .. } | DirCommand::SetPermissions(_)) => {
            Some(RequestKey::Directories)
        }
        AppCommand::Sessions(
            SessionCommand::Resume { .. }
            | SessionCommand::Archive { .. }
            | SessionCommand::CreateAndEnter { .. },
        ) => Some(RequestKey::Sessions),
        AppCommand::Sessions(SessionCommand::SwitchThread { .. })
        | AppCommand::Thread(
            ThreadCommand::ExecuteProductCommand(_)
            | ThreadCommand::LoadOlderHistory
            | ThreadCommand::OpenRewindPicker
            | ThreadCommand::RewindToCheckpoint { .. }
            | ThreadCommand::SubmitTurn { .. }
            | ThreadCommand::SubmitQueuedTurn { .. }
            | ThreadCommand::SteerTurn { .. },
        ) => Some(RequestKey::Thread),
        AppCommand::Mcp(_) => Some(RequestKey::Mcp),
        AppCommand::Skills(_) => Some(RequestKey::Skills),
    }
}
