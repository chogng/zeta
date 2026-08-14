use zeta_remote_connections::RemoteConnectionName;

use super::ManagerStatus;
use super::RemoteConnectionManagerState;

impl RemoteConnectionManagerState {
    pub(crate) fn is_launching(&self) -> bool {
        self.open
            .as_ref()
            .is_some_and(|open| open.launching.is_some())
    }

    pub(crate) fn connect_request(&mut self) -> Option<RemoteConnectionName> {
        let Some(open) = self.open.as_ref() else {
            return None;
        };
        if open.dirty {
            self.set_error("Save changes before connecting");
            return None;
        }
        if open.launching.is_some() {
            self.set_error("The Remote window is already preparing");
            return None;
        }
        let Some(name) = open.original.clone() else {
            self.set_error("Save the new connection before connecting");
            return None;
        };
        Some(name)
    }

    pub(crate) fn can_delete(&self) -> bool {
        self.selected_name().is_some() && !self.is_launching()
    }

    pub(crate) fn can_mutate(&self) -> bool {
        self.is_open() && !self.is_launching()
    }

    pub(crate) fn can_connect(&self) -> bool {
        self.open
            .as_ref()
            .is_some_and(|open| open.original.is_some() && !open.dirty && open.launching.is_none())
    }

    pub(crate) fn launch_started(&mut self, name: RemoteConnectionName) {
        if let Some(open) = self.open.as_mut() {
            open.launching = Some(name);
            open.delete_confirmation = None;
            open.status = Some(ManagerStatus {
                message: "Starting Remote window…".into(),
                error: false,
            });
        }
    }

    pub(crate) fn launch_progress(&mut self, message: impl Into<String>) {
        if let Some(open) = self.open.as_mut()
            && open.launching.is_some()
        {
            open.status = Some(ManagerStatus {
                message: message.into(),
                error: false,
            });
        }
    }

    pub(crate) fn launch_failed(&mut self, error: impl Into<String>) {
        if let Some(open) = self.open.as_mut() {
            open.launching = None;
            open.status = Some(ManagerStatus {
                message: error.into(),
                error: true,
            });
        }
    }
}
