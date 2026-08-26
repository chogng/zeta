use std::path::Path;
use std::path::PathBuf;

use zeta_protocol::Session;
use zeta_protocol::SessionId;

/// Stable logical identity for one input that can be shown by a Workbench tab.
///
/// UI element identities deliberately do not belong here. They are allocated by the mounted tab
/// list when the input is projected into a frame.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) enum TabInputKey {
    Session(SessionId),
    Settings,
}

impl TabInputKey {
    pub(crate) fn session(session_id: SessionId) -> Self {
        Self::Session(session_id)
    }

    pub(crate) fn session_id(&self) -> Option<&SessionId> {
        match self {
            Self::Session(session_id) => Some(session_id),
            Self::Settings => None,
        }
    }

    pub(crate) const fn is_session(&self) -> bool {
        matches!(self, Self::Session(_))
    }

    pub(crate) const fn is_settings(&self) -> bool {
        matches!(self, Self::Settings)
    }
}

/// Product-owned logical input behind one Workbench tab.
///
/// This record contains the stable input identity and the labels needed by the shell projection.
/// Session lifecycle and Thread state remain owned by the App Server session adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TabInput {
    key: TabInputKey,
    title: String,
    workspace: String,
    workspace_root: Option<PathBuf>,
    status_label: String,
}

impl TabInput {
    pub(crate) fn from_settings() -> Self {
        Self {
            key: TabInputKey::Settings,
            title: "Settings".to_owned(),
            workspace: "Application".to_owned(),
            workspace_root: None,
            status_label: String::new(),
        }
    }

    pub(crate) fn from_session(session: &Session, workspace: &str) -> Self {
        Self {
            key: TabInputKey::session(session.session_id.clone()),
            title: session.title.clone(),
            workspace: workspace_label(session, workspace),
            workspace_root: session
                .workspace
                .as_ref()
                .map(|binding| binding.root.clone()),
            status_label: "Active".to_owned(),
        }
    }

    pub(crate) fn key(&self) -> &TabInputKey {
        &self.key
    }

    pub(crate) fn session_id(&self) -> Option<&SessionId> {
        self.key.session_id()
    }

    pub(crate) const fn is_session(&self) -> bool {
        self.key.is_session()
    }

    pub(crate) const fn is_settings(&self) -> bool {
        self.key.is_settings()
    }

    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    pub(crate) fn workspace(&self) -> &str {
        &self.workspace
    }

    pub(crate) fn workspace_root(&self) -> Option<&Path> {
        self.workspace_root.as_deref()
    }

    pub(crate) fn status_label(&self) -> &str {
        &self.status_label
    }

    fn update_from_session(&mut self, session: &Session, workspace: &str) {
        debug_assert_eq!(self.session_id(), Some(&session.session_id));
        self.title = session.title.clone();
        self.workspace = workspace_label(session, workspace);
        self.workspace_root = session
            .workspace
            .as_ref()
            .map(|binding| binding.root.clone());
        self.status_label = "Active".to_owned();
    }

    pub(crate) fn update_status(&mut self, status_label: impl Into<String>) {
        self.status_label = status_label.into();
    }
}

/// A change made while inserting or refreshing one logical TabInput.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum TabInputChange {
    Added(TabInputKey),
    Updated(TabInputKey),
}

/// Product-side collection and active selection for Workbench inputs.
///
/// The model owns logical ordering and selection only. It does not allocate UI identities, paint
/// tabs, or perform activation side effects such as switching the App Server subscription or a
/// terminal pane.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TabInputModel {
    inputs: Vec<TabInput>,
    /// The input whose content is currently mounted in the main workbench part.
    active: Option<TabInputKey>,
    /// The last Session selection, retained while the singleton Settings input is active.
    last_session: Option<SessionId>,
}

impl Default for TabInputModel {
    fn default() -> Self {
        Self {
            inputs: vec![TabInput::from_settings()],
            active: None,
            last_session: None,
        }
    }
}

impl TabInputModel {
    pub(crate) fn inputs(&self) -> &[TabInput] {
        &self.inputs
    }

    pub(crate) fn session_count(&self) -> usize {
        self.inputs
            .iter()
            .filter(|input| input.is_session())
            .count()
    }

    pub(crate) fn session_input_at(&self, index: usize) -> Option<&TabInput> {
        self.inputs
            .iter()
            .filter(|input| input.is_session())
            .nth(index)
    }

    pub(crate) fn active_key(&self) -> Option<&TabInputKey> {
        self.active.as_ref()
    }

    pub(crate) fn selected_session(&self) -> Option<&SessionId> {
        self.last_session.as_ref()
    }

    pub(crate) const fn is_settings(&self) -> bool {
        matches!(self.active, Some(TabInputKey::Settings))
    }

    pub(crate) fn activate(&mut self, key: TabInputKey) -> bool {
        if self.inputs.iter().any(|input| input.key() == &key) {
            if let Some(session_id) = key.session_id() {
                self.last_session = Some(session_id.clone());
            }
            self.active = Some(key);
            true
        } else {
            false
        }
    }

    pub(crate) fn activate_session(&mut self, session_id: &SessionId) -> bool {
        self.activate(TabInputKey::session(session_id.clone()))
    }

    pub(crate) fn activate_settings(&mut self) -> bool {
        self.activate(TabInputKey::Settings)
    }

    pub(crate) fn activate_last_session(&mut self) -> bool {
        let Some(session_id) = self.last_session.clone() else {
            self.active = None;
            return false;
        };
        self.activate_session(&session_id)
    }

    pub(crate) fn upsert_session(&mut self, session: &Session, workspace: &str) -> TabInputChange {
        let key = TabInputKey::session(session.session_id.clone());
        let was_settings = self.is_settings();
        if let Some(input) = self.inputs.iter_mut().find(|input| input.key() == &key) {
            input.update_from_session(session, workspace);
            self.last_session = Some(session.session_id.clone());
            if !was_settings {
                self.active = Some(key.clone());
            }
            return TabInputChange::Updated(key);
        }

        let insertion_index = self
            .inputs
            .iter()
            .position(TabInput::is_settings)
            .unwrap_or(self.inputs.len());
        self.inputs
            .insert(insertion_index, TabInput::from_session(session, workspace));
        self.last_session = Some(session.session_id.clone());
        if !was_settings {
            self.active = Some(key.clone());
        }
        TabInputChange::Added(key)
    }

    pub(crate) fn upsert_catalog_session(
        &mut self,
        session: &Session,
        workspace: &str,
    ) -> TabInputChange {
        let key = TabInputKey::session(session.session_id.clone());
        if let Some(input) = self.inputs.iter_mut().find(|input| input.key() == &key) {
            input.update_from_session(session, workspace);
            return TabInputChange::Updated(key);
        }

        let insertion_index = self
            .inputs
            .iter()
            .position(TabInput::is_settings)
            .unwrap_or(self.inputs.len());
        self.inputs
            .insert(insertion_index, TabInput::from_session(session, workspace));
        TabInputChange::Added(key)
    }

    pub(crate) fn update_status(&mut self, session_id: &SessionId, status_label: &str) {
        if let Some(input) = self
            .inputs
            .iter_mut()
            .find(|input| input.session_id() == Some(session_id))
        {
            input.update_status(status_label);
        }
    }
}

fn workspace_label<'a>(session: &'a Session, fallback: &'a str) -> String {
    session
        .workspace
        .as_ref()
        .and_then(|binding| binding.root.file_name())
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| fallback.to_owned())
}

#[cfg(test)]
#[path = "tab_input_tests.rs"]
mod tests;
