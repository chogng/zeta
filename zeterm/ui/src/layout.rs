//! Zeterm product pane topology built on backend-neutral [`zui`] layout contracts.

mod root;
mod session_workspace;
mod terminal_workspace;

/// Visibility projected by a host into a sidebar layout request.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum SidebarVisibility {
    /// Do not include the sidebar leaf in the resolved layout.
    #[default]
    Collapsed,
    /// Include the sidebar when the available width can preserve both panes.
    Expanded,
}

pub use root::InspectorPane;
pub use root::LogicalViewport;
pub use root::RootLayout;
pub use session_workspace::SessionSidebarLayout;
pub use session_workspace::SessionSidebarLayoutSpec;
pub use terminal_workspace::SidebarLayoutSpec;
pub use terminal_workspace::TerminalWorkspaceLayout;
