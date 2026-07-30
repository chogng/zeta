use zeta_ui::{
    GridLayout, GridNode, GridPane, Rect, SplitViewLayoutPriority, SplitViewOrientation,
    SplitViewPane,
};

use crate::agent_sidebar::AgentSidebarState;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum WorkspaceLeafId {
    ActiveTerminal,
    AgentSidebar,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum WorkspaceSplitId {
    Root,
}

/// Resolved Terminal Workspace and Agent Sidebar geometry.
///
/// The current runtime contributes one active terminal leaf and an optional
/// Agent Sidebar leaf. Future multi-terminal ownership may replace the active
/// leaf with nested split nodes without changing ShellLayout's outer Sessions
/// sidebar contract.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TerminalWorkspaceLayout {
    active_pane_bounds: Rect,
    agent_sidebar_bounds: Option<Rect>,
}

impl TerminalWorkspaceLayout {
    pub(crate) fn for_bounds(bounds: Rect, agent_sidebar: AgentSidebarState) -> Self {
        let sidebar_is_visible = agent_sidebar.is_visible_for(bounds.size.width);
        let active_preferred_width = if sidebar_is_visible {
            (bounds.size.width - agent_sidebar.preferred_width()).max(0.0)
        } else {
            bounds.size.width
        };
        let root = GridNode::split(
            WorkspaceSplitId::Root,
            SplitViewOrientation::Horizontal,
            vec![
                GridPane::new(
                    GridNode::leaf(WorkspaceLeafId::ActiveTerminal),
                    SplitViewPane::new(
                        active_preferred_width,
                        agent_sidebar.minimum_main_width(),
                        f32::INFINITY,
                    )
                    .with_priority(SplitViewLayoutPriority::High),
                ),
                GridPane::new(
                    GridNode::leaf(WorkspaceLeafId::AgentSidebar),
                    agent_sidebar.pane_sizing(bounds.size.width),
                ),
            ],
        );
        let layout = GridLayout::new(bounds, &root);
        let active_pane_bounds = layout
            .leaf(WorkspaceLeafId::ActiveTerminal)
            .expect("Terminal Workspace Grid must retain its active leaf")
            .bounds();
        let agent_sidebar_bounds = layout
            .leaf(WorkspaceLeafId::AgentSidebar)
            .map(|leaf| leaf.bounds());
        Self {
            active_pane_bounds,
            agent_sidebar_bounds,
        }
    }

    pub(crate) const fn active_pane_bounds(self) -> Rect {
        self.active_pane_bounds
    }

    pub(crate) const fn agent_sidebar_bounds(self) -> Option<Rect> {
        self.agent_sidebar_bounds
    }
}

#[cfg(test)]
#[path = "terminal_workspace_layout_tests.rs"]
mod tests;
