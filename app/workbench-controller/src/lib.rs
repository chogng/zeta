//! Product Workbench coordination and terminal-pane binding.

mod binding;

pub use binding::PaneBinding;
pub use zeta_workbench_host::InspectorPartState;
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

/// Product coordinator combining logical Workbench state with pane runtime bindings.
pub struct WorkbenchController {
    host: zeta_workbench_host::WorkbenchHost<PaneBinding>,
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
        }
    }

    pub const fn workbench(&self) -> &zeta_workbench_host::Workbench {
        self.host.workbench()
    }

    pub const fn workbench_mut(&mut self) -> &mut zeta_workbench_host::Workbench {
        self.host.workbench_mut()
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
        self.host.close_tab(tab_key)
    }
}
