use zeta_ui::{
    GridLayout, GridNode, GridPane, Rect, SplitViewLayoutPriority, SplitViewOrientation,
    SplitViewPane,
};

const EXPLORER_PREFERRED_HEIGHT: f32 = 180.0;
const EXPLORER_MINIMUM_HEIGHT: f32 = 96.0;
const EDITOR_MINIMUM_HEIGHT: f32 = 160.0;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum AgentSidebarLeafId {
    Explorer,
    Editor,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum AgentSidebarSplitId {
    Root,
}

/// Resolved sibling Pane geometry inside the Agent Sidebar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AgentSidebarLayout {
    explorer: Rect,
    editor: Rect,
}

impl AgentSidebarLayout {
    pub(crate) fn for_bounds(bounds: Rect) -> Self {
        let editor_preferred_height =
            (bounds.size.height - EXPLORER_PREFERRED_HEIGHT).max(EDITOR_MINIMUM_HEIGHT);
        let root = GridNode::split(
            AgentSidebarSplitId::Root,
            SplitViewOrientation::Vertical,
            vec![
                GridPane::new(
                    GridNode::leaf(AgentSidebarLeafId::Explorer),
                    SplitViewPane::new(
                        EXPLORER_PREFERRED_HEIGHT,
                        EXPLORER_MINIMUM_HEIGHT,
                        f32::INFINITY,
                    ),
                ),
                GridPane::new(
                    GridNode::leaf(AgentSidebarLeafId::Editor),
                    SplitViewPane::new(
                        editor_preferred_height,
                        EDITOR_MINIMUM_HEIGHT,
                        f32::INFINITY,
                    )
                    .with_priority(SplitViewLayoutPriority::High),
                ),
            ],
        );
        let layout = GridLayout::new(bounds, &root);
        Self {
            explorer: layout
                .leaf(AgentSidebarLeafId::Explorer)
                .expect("Agent Sidebar Grid must retain ExplorerPane")
                .bounds(),
            editor: layout
                .leaf(AgentSidebarLeafId::Editor)
                .expect("Agent Sidebar Grid must retain EditorPane")
                .bounds(),
        }
    }

    pub(crate) const fn explorer(self) -> Rect {
        self.explorer
    }

    pub(crate) const fn editor(self) -> Rect {
        self.editor
    }
}

#[cfg(test)]
#[path = "agent_sidebar_layout_tests.rs"]
mod tests;
