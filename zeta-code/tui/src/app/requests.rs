use super::App;
use super::AppCommand;
use super::AppEvent;
use super::completion::Completion;
use crate::client;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RequestLane {
    Control,
    Write,
    Read,
    Host,
}

#[derive(Default)]
pub(super) struct RequestTasks {
    control: Option<client::RequestTask<Completion>>,
    write: Option<client::RequestTask<Completion>>,
    read: Option<client::RequestTask<Completion>>,
    host: Option<client::RequestTask<Completion>>,
}

impl RequestTasks {
    pub(super) fn is_idle(&self, lane: Option<RequestLane>) -> bool {
        lane.is_none_or(|lane| self.task(lane).is_none())
    }

    pub(super) fn spawn(
        &mut self,
        lane: Option<RequestLane>,
        name: &'static str,
        request: impl FnOnce() -> Completion + Send + 'static,
        app: &mut App,
    ) {
        let Some(lane) = lane else {
            app.update(AppEvent::FailureReported(format!(
                "background request `{name}` has no request lane"
            )));
            return;
        };
        debug_assert!(self.task(lane).is_none());
        match client::RequestTask::spawn(name, request) {
            Ok(task) => *self.task_mut(lane) = Some(task),
            Err(error) => app.update(AppEvent::FailureReported(format!(
                "could not start background request: {error}"
            ))),
        }
    }

    pub(super) fn poll(&mut self) -> Vec<Result<Completion, std::io::Error>> {
        let mut completed = Vec::new();
        for lane in [
            RequestLane::Control,
            RequestLane::Write,
            RequestLane::Read,
            RequestLane::Host,
        ] {
            let result = match self.task_mut(lane).as_mut() {
                Some(task) => task.poll(),
                None => continue,
            };
            match result {
                Ok(Some(completion)) => {
                    *self.task_mut(lane) = None;
                    completed.push(Ok(completion));
                }
                Ok(None) => {}
                Err(error) => {
                    *self.task_mut(lane) = None;
                    completed.push(Err(error));
                }
            }
        }
        completed
    }

    fn task(&self, lane: RequestLane) -> &Option<client::RequestTask<Completion>> {
        match lane {
            RequestLane::Control => &self.control,
            RequestLane::Write => &self.write,
            RequestLane::Read => &self.read,
            RequestLane::Host => &self.host,
        }
    }

    fn task_mut(&mut self, lane: RequestLane) -> &mut Option<client::RequestTask<Completion>> {
        match lane {
            RequestLane::Control => &mut self.control,
            RequestLane::Write => &mut self.write,
            RequestLane::Read => &mut self.read,
            RequestLane::Host => &mut self.host,
        }
    }
}

pub(super) fn request_lane(command: &AppCommand) -> Option<RequestLane> {
    match command {
        AppCommand::Interrupt | AppCommand::ResolveThreadRequest(_) => Some(RequestLane::Control),
        AppCommand::OpenConfigEditor
        | AppCommand::OpenKeymapEditor
        | AppCommand::OpenStatusLineEditor
        | AppCommand::OpenThemePicker
        | AppCommand::LoadOlderHistory
        | AppCommand::OpenCustomThemePicker
        | AppCommand::OpenRewindPicker => Some(RequestLane::Read),
        AppCommand::CopyLastResponse
        | AppCommand::ExportTranscript { .. }
        | AppCommand::ReadClipboardImage
        | AppCommand::RefreshClipboardImageAvailability => Some(RequestLane::Host),
        AppCommand::Quit | AppCommand::Suspend | AppCommand::CycleNextApprovalMode => None,
        AppCommand::ExecuteProductCommand(_)
        | AppCommand::EditKeymap(_)
        | AppCommand::EditConfig(_)
        | AppCommand::SetProviderApiKey(_)
        | AppCommand::EditStatusLine(_)
        | AppCommand::ConnectConnectorDeviceOAuth { .. }
        | AppCommand::DisconnectConnector { .. }
        | AppCommand::RemoveDir { .. }
        | AppCommand::SetDirPermissions(_)
        | AppCommand::RewindToCheckpoint { .. }
        | AppCommand::ResumeSession { .. }
        | AppCommand::ArchiveSessions { .. }
        | AppCommand::CreateSessionAndEnter { .. }
        | AppCommand::SwitchThread { .. }
        | AppCommand::SetMcpEnablement { .. }
        | AppCommand::SetPreferredModel { .. }
        | AppCommand::SetCustomTheme { .. }
        | AppCommand::SetTheme { .. }
        | AppCommand::SetSkillEnablement { .. }
        | AppCommand::SubmitTurn { .. }
        | AppCommand::SubmitQueuedTurn { .. }
        | AppCommand::SteerTurn { .. } => Some(RequestLane::Write),
    }
}
