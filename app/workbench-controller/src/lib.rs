//! Product Workbench coordination and terminal-pane binding.

use std::collections::HashMap;

mod binding;

pub use binding::PaneBinding;
pub use zeta_workbench_host::PaneContainer;
pub use zeta_workbench_host::PaneGroupId;
pub use zeta_workbench_host::PaneHost;
pub use zeta_workbench_host::PaneHostScope;
pub use zeta_workbench_host::PaneInput;
pub use zeta_workbench_host::PaneInputKind;
pub use zeta_workbench_host::PaneMount;
pub use zeta_workbench_host::PanePart;
pub use zeta_workbench_host::PaneSplitDirection;
pub use zeta_workbench_host::PaneSplitId;
pub use zeta_workbench_host::TabGroupId;
pub use zeta_workbench_host::TabInput;
pub use zeta_workbench_host::TabInputChange;
pub use zeta_workbench_host::TabInputKey;
pub use zeta_workbench_host::TabPart;
pub use zeta_workbench_ui::InspectorPartState;

use zeta_protocol::Session;
use zeta_protocol::SessionId;
use zeta_workbench_host::Pane;
use zeta_workbench_host::PaneInputId;
use zeta_workbench_host::TabInputMetadata;

/// Product coordinator combining logical Workbench state with pane runtime bindings.
pub struct WorkbenchController {
    host: zeta_workbench_host::WorkbenchHost<PaneBinding>,
    workspace_returns: HashMap<TabInputKey, PaneInput>,
}

impl Default for WorkbenchController {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkbenchController {
    pub fn new() -> Self {
        Self {
            host: zeta_workbench_host::WorkbenchHost::new(),
            workspace_returns: HashMap::new(),
        }
    }

    pub const fn workbench(&self) -> &zeta_workbench_host::Workbench {
        self.host.workbench()
    }

    pub fn workbench_mut(&mut self) -> WorkbenchCommands<'_> {
        let Self {
            host,
            workspace_returns,
        } = self;
        WorkbenchCommands {
            workbench: host.workbench_mut(),
            workspace_returns,
        }
    }

    pub const fn pane_host(&self) -> &PaneHost<PaneBinding> {
        self.host.pane_host()
    }

    pub const fn pane_host_mut(&mut self) -> &mut PaneHost<PaneBinding> {
        self.host.pane_host_mut()
    }

    pub fn close_tab(
        &mut self,
        tab_key: &TabInputKey,
    ) -> Option<(zeta_workbench_host::ClosedTab, Vec<PaneBinding>)> {
        let closed = self.host.close_tab(tab_key)?;
        self.workspace_returns.remove(tab_key);
        Some(closed)
    }
}

/// Product command boundary for mutating Workbench state.
///
/// Logical Tab/Pane transitions remain owned by `zeta-workbench`; Session conversion, default
/// content selection, and cross-feature return state are coordinated here.
pub struct WorkbenchCommands<'a> {
    workbench: &'a mut zeta_workbench_host::Workbench,
    workspace_returns: &'a mut HashMap<TabInputKey, PaneInput>,
}

impl WorkbenchCommands<'_> {
    pub fn tab_part_mut(&mut self) -> TabPartCommands<'_> {
        TabPartCommands {
            workbench: self.workbench,
        }
    }

    pub fn ensure_root_pane(&mut self, tab_key: TabInputKey, input: PaneInput) -> PaneGroupId {
        self.workbench.ensure_root_pane(tab_key, input)
    }

    pub fn activate_session(&mut self, session_id: &SessionId) -> bool {
        self.workbench.activate_session(session_id)
    }

    pub fn activate_settings(&mut self) -> bool {
        self.workbench.activate_settings()
    }

    pub fn upsert_session(&mut self, session: &Session, workspace: &str) -> TabInputChange {
        let tab_input = session_tab_input(session, workspace);
        self.workbench
            .upsert_session_input(tab_input, PaneInput::terminal(session.session_id.clone()))
    }

    pub fn upsert_catalog_session(&mut self, session: &Session, workspace: &str) -> TabInputChange {
        let tab_input = session_tab_input(session, workspace);
        self.workbench.upsert_catalog_session_input(
            tab_input,
            PaneInput::terminal(session.session_id.clone()),
        )
    }

    pub fn create_pane_with_direction(
        &mut self,
        input: PaneInput,
        direction: PaneSplitDirection,
    ) -> Option<PaneGroupId> {
        self.workbench.create_pane_with_direction(input, direction)
    }

    pub fn destroy_pane(&mut self) -> Option<Vec<Pane>> {
        self.workbench.destroy_pane()
    }

    pub fn activate_pane(&mut self, tab_key: &TabInputKey, pane_id: PaneGroupId) -> bool {
        self.workbench.activate_pane(tab_key, pane_id)
    }

    pub fn mount_input(
        &mut self,
        tab_key: &TabInputKey,
        pane_id: PaneGroupId,
        input: PaneInput,
    ) -> Option<PaneInput> {
        self.workbench.mount_input(tab_key, pane_id, input)
    }

    pub fn open_input(
        &mut self,
        tab_key: &TabInputKey,
        pane_id: PaneGroupId,
        input: PaneInput,
    ) -> Option<PaneInputId> {
        self.workbench.open_input(tab_key, pane_id, input)
    }

    pub fn focus_next_pane(&mut self, tab_key: &TabInputKey) -> Option<PaneGroupId> {
        self.workbench.focus_next_pane(tab_key)
    }

    pub fn focus_previous_pane(&mut self, tab_key: &TabInputKey) -> Option<PaneGroupId> {
        self.workbench.focus_previous_pane(tab_key)
    }

    pub fn resize_split(
        &mut self,
        tab_key: &TabInputKey,
        split_id: PaneSplitId,
        ratio: f32,
    ) -> bool {
        self.workbench.resize_split(tab_key, split_id, ratio)
    }

    pub fn remember_workspace_return(&mut self, tab_key: &TabInputKey, input: PaneInput) -> bool {
        if self.workbench.pane_container(tab_key).is_none() {
            return false;
        }
        self.workspace_returns.insert(tab_key.clone(), input);
        true
    }

    pub fn take_workspace_return(&mut self, tab_key: &TabInputKey) -> Option<PaneInput> {
        self.workspace_returns.remove(tab_key)
    }

    pub fn clear_workspace_return(&mut self, tab_key: &TabInputKey) -> bool {
        if self.workbench.pane_container(tab_key).is_none() {
            return false;
        }
        self.workspace_returns.remove(tab_key);
        true
    }
}

/// Restricted Tab commands that cannot bypass Workbench's Tab-to-container invariant.
pub struct TabPartCommands<'a> {
    workbench: &'a mut zeta_workbench_host::Workbench,
}

impl TabPartCommands<'_> {
    pub fn update_status(&mut self, session_id: &SessionId, status_label: &str) {
        self.workbench
            .update_session_status(session_id, status_label);
    }

    pub fn activate_last_session(&mut self) -> bool {
        self.workbench.activate_last_session()
    }
}

fn session_tab_input(session: &Session, workspace_label: &str) -> TabInput {
    let workspace = session
        .workspace
        .as_ref()
        .and_then(|binding| binding.root.file_name())
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or(workspace_label);
    let mut metadata = TabInputMetadata::new(&session.title, workspace).with_status_label("Active");
    if let Some(workspace_root) = session
        .workspace
        .as_ref()
        .map(|binding| binding.root.clone())
    {
        metadata = metadata.with_workspace_root(workspace_root);
    }
    TabInput::session(session.session_id.clone(), metadata)
}

#[cfg(test)]
#[path = "controller_tests.rs"]
mod tests;
