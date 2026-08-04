//! Product composition geometry built on the backend-neutral [`zui`] layout contracts.
//!
//! This crate owns zeterm's stable window, inspector, and terminal-workspace topology. It does
//! not own product state, command dispatch, renderer resources, or platform event routing. Hosts
//! translate their state into the small layout specifications exposed here, then consume the
//! resolved bounds and resize snapshots.

mod root;
mod terminal_workspace;

pub use root::InspectorPane;
pub use root::LogicalViewport;
pub use root::RootLayout;
pub use terminal_workspace::SidebarLayoutSpec;
pub use terminal_workspace::SidebarVisibility;
pub use terminal_workspace::TerminalWorkspaceLayout;
