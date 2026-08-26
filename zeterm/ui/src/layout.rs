//! Zeterm product pane topology built on backend-neutral [`zui`] layout contracts.
//!
//! The layout types resolve structural Part/Pane geometry only. Product hosts retain content,
//! identity, focus semantics, event routing, and runtime state.

mod pane_group;
mod root;
mod session_workspace;
mod terminal_workspace;
mod workbench;

/// Visibility projected by a host into a sidebar layout request.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum SidebarVisibility {
    /// Do not include the sidebar leaf in the resolved layout.
    #[default]
    Collapsed,
    /// Include the sidebar when the available width can preserve both panes.
    Expanded,
}

pub use pane_group::PaneGroupLayout;
pub use root::InspectorPane;
pub use root::LogicalViewport;
pub use root::RootLayout;
pub use session_workspace::SessionSidebarLayout;
pub use session_workspace::SessionSidebarLayoutSpec;
pub use terminal_workspace::SidebarLayoutSpec;
pub use terminal_workspace::TerminalWorkspaceLayout;
pub use workbench::WorkbenchLayout;
pub use workbench::WorkbenchLayoutSpec;
pub use workbench::WorkbenchPart;
