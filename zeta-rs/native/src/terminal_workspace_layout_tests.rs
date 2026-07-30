use super::TerminalWorkspaceLayout;
use crate::agent_sidebar::AgentSidebarState;
use zeta_ui::Rect;

#[test]
fn current_terminal_workspace_projects_its_active_leaf_through_grid_layout() {
    let bounds = Rect::from_xywh(200.0, 32.0, 800.0, 668.0);

    let layout = TerminalWorkspaceLayout::for_bounds(bounds, AgentSidebarState::default());

    assert_eq!(layout.active_pane_bounds(), bounds);
    assert_eq!(layout.agent_sidebar_bounds(), None);
}

#[test]
fn expanded_agent_sidebar_is_the_rightmost_grid_leaf() {
    let bounds = Rect::from_xywh(200.0, 32.0, 800.0, 668.0);

    let layout = TerminalWorkspaceLayout::for_bounds(bounds, AgentSidebarState::expanded());

    assert_eq!(
        layout.active_pane_bounds(),
        Rect::from_xywh(200.0, 32.0, 480.0, 668.0)
    );
    assert_eq!(
        layout.agent_sidebar_bounds(),
        Some(Rect::from_xywh(680.0, 32.0, 320.0, 668.0))
    );
}

#[test]
fn constrained_grid_omits_the_agent_sidebar_leaf() {
    let bounds = Rect::from_xywh(0.0, 32.0, 559.0, 668.0);

    let layout = TerminalWorkspaceLayout::for_bounds(bounds, AgentSidebarState::expanded());

    assert_eq!(layout.active_pane_bounds(), bounds);
    assert_eq!(layout.agent_sidebar_bounds(), None);
}
