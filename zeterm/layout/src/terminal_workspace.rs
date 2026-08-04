use zui::GridLayout;
use zui::GridNode;
use zui::GridPane;
use zui::Rect;
use zui::SplitViewLayoutPriority;
use zui::SplitViewOrientation;
use zui::SplitViewPane;
use zui::SplitViewResizeSnapshot;

/// Visibility projected by a host into a workspace sidebar layout request.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum SidebarVisibility {
    /// Do not include the sidebar leaf in the resolved layout.
    #[default]
    Collapsed,
    /// Include the sidebar when the available width can preserve both panes.
    Expanded,
}

/// Host-neutral sizing policy for one right-hand workspace sidebar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SidebarLayoutSpec {
    visibility: SidebarVisibility,
    preferred_width: f32,
    minimum_width: f32,
    maximum_width: f32,
    minimum_main_width: f32,
}

impl SidebarLayoutSpec {
    /// Creates a sidebar sizing policy. The host owns visibility state and persisted width; this
    /// value only projects them into layout geometry for one frame.
    pub const fn new(
        visibility: SidebarVisibility,
        preferred_width: f32,
        minimum_width: f32,
        maximum_width: f32,
        minimum_main_width: f32,
    ) -> Self {
        Self {
            visibility,
            preferred_width,
            minimum_width,
            maximum_width,
            minimum_main_width,
        }
    }

    /// Returns whether this policy can display the sidebar in the available width.
    pub const fn is_visible_for(self, available_width: f32) -> bool {
        matches!(self.visibility, SidebarVisibility::Expanded)
            && available_width >= self.minimum_width + self.minimum_main_width
    }

    /// Returns the preferred sidebar width before layout constraints are applied.
    pub const fn preferred_width(self) -> f32 {
        self.preferred_width
    }

    /// Returns the minimum width reserved for the main workspace pane.
    pub const fn minimum_main_width(self) -> f32 {
        self.minimum_main_width
    }

    /// Converts this policy into the generic split-pane sizing consumed by [`zui`].
    pub fn pane_sizing(self, available_width: f32) -> SplitViewPane {
        let sidebar =
            SplitViewPane::new(self.preferred_width, self.minimum_width, self.maximum_width);
        if self.is_visible_for(available_width) {
            sidebar
        } else {
            sidebar.hidden()
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum WorkspaceLeafId {
    ActiveTerminal,
    AgentSidebar,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum WorkspaceSplitId {
    Root,
}

/// Resolved terminal workspace and optional right-hand sidebar geometry.
///
/// This type owns only topology and resize geometry. The host retains ownership of terminal,
/// agent, or editor state and may use the returned bounds to compose those domains.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerminalWorkspaceLayout {
    active_pane_bounds: Rect,
    sidebar_bounds: Option<Rect>,
    sidebar_sash_track: Option<Rect>,
    sidebar_resize_snapshot: Option<SplitViewResizeSnapshot>,
}

impl TerminalWorkspaceLayout {
    /// Resolves the active workspace and optional sidebar from a host-neutral sizing policy.
    pub fn for_bounds(bounds: Rect, sidebar: SidebarLayoutSpec) -> Self {
        let sidebar_is_visible = sidebar.is_visible_for(bounds.size.width);
        let active_preferred_width = if sidebar_is_visible {
            (bounds.size.width - sidebar.preferred_width()).max(0.0)
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
                        sidebar.minimum_main_width(),
                        f32::INFINITY,
                    )
                    .with_priority(SplitViewLayoutPriority::High),
                ),
                GridPane::new(
                    GridNode::leaf(WorkspaceLeafId::AgentSidebar),
                    sidebar.pane_sizing(bounds.size.width),
                ),
            ],
        );
        let layout = GridLayout::new(bounds, &root);
        let sidebar_sash = layout.sashes().first().copied();
        let active_pane_bounds = layout
            .leaf(WorkspaceLeafId::ActiveTerminal)
            .expect("Terminal Workspace Grid must retain its active leaf")
            .bounds();
        let sidebar_bounds = layout
            .leaf(WorkspaceLeafId::AgentSidebar)
            .map(|leaf| leaf.bounds());
        Self {
            active_pane_bounds,
            sidebar_bounds,
            sidebar_sash_track: sidebar_sash.map(|sash| sash.track_bounds()),
            sidebar_resize_snapshot: sidebar_sash.map(|sash| sash.resize_snapshot()),
        }
    }

    /// Returns the active terminal/editor pane bounds.
    pub const fn active_pane_bounds(self) -> Rect {
        self.active_pane_bounds
    }

    /// Returns the optional sidebar bounds.
    pub const fn sidebar_bounds(self) -> Option<Rect> {
        self.sidebar_bounds
    }

    /// Returns the sash track used to paint and hit-test the sidebar divider.
    pub const fn sidebar_sash_track(self) -> Option<Rect> {
        self.sidebar_sash_track
    }

    /// Returns the resize snapshot matching the resolved sidebar sash.
    pub const fn sidebar_resize_snapshot(self) -> Option<SplitViewResizeSnapshot> {
        self.sidebar_resize_snapshot
    }
}

#[cfg(test)]
#[path = "terminal_workspace_tests.rs"]
mod tests;
