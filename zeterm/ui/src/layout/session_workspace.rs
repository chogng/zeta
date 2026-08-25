use zui::ui::Rect;
use zui::ui::SplitViewLayout;
use zui::ui::SplitViewLayoutPriority;
use zui::ui::SplitViewOrientation;
use zui::ui::SplitViewPane;
use zui::ui::SplitViewResizeSnapshot;

use super::SidebarVisibility;

const SESSION_PANE_INDEX: usize = 0;
const MAIN_PANE_INDEX: usize = 1;

/// Host-neutral sizing policy for the Sessions Part.
///
/// The host retains visibility, preferred width, and resize state; this value only projects that
/// state into one frame's split geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SessionSidebarLayoutSpec {
    visibility: SidebarVisibility,
    preferred_width: f32,
    minimum_width: f32,
    maximum_width: f32,
    minimum_main_width: f32,
}

impl SessionSidebarLayoutSpec {
    /// Creates a sizing policy for the Sessions Part.
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

    /// Returns whether the Sessions Part can be displayed without violating the main minimum.
    pub const fn is_visible_for(self, available_width: f32) -> bool {
        matches!(self.visibility, SidebarVisibility::Expanded)
            && available_width >= self.minimum_width + self.minimum_main_width
    }

    /// Resolves the Sessions Part and main Part bounds for one host viewport.
    pub fn for_bounds(self, bounds: Rect) -> SessionSidebarLayout {
        let sidebar_is_visible = self.is_visible_for(bounds.size.width);
        let sidebar =
            SplitViewPane::new(self.preferred_width, self.minimum_width, self.maximum_width);
        let sidebar = if sidebar_is_visible {
            sidebar
        } else {
            sidebar.hidden()
        };
        let main_preferred_width = if sidebar_is_visible {
            (bounds.size.width - self.preferred_width).max(0.0)
        } else {
            bounds.size.width
        };
        let main = SplitViewPane::new(main_preferred_width, self.minimum_main_width, f32::INFINITY)
            .with_priority(SplitViewLayoutPriority::High);
        let layout =
            SplitViewLayout::new(bounds, SplitViewOrientation::Horizontal, &[sidebar, main]);
        let sessions_bounds = layout
            .pane_bounds(SESSION_PANE_INDEX)
            .filter(|bounds| !bounds.is_empty());
        let main_bounds = layout
            .pane_bounds(MAIN_PANE_INDEX)
            .expect("Sessions split must retain its main pane");
        let sash = layout.sash(0);
        SessionSidebarLayout {
            sessions_bounds,
            main_bounds,
            sash_track: sash.map(|sash| sash.track_bounds()),
            resize_snapshot: sash.map(|sash| sash.resize_snapshot()),
        }
    }
}

/// Resolved Sessions Part and main Part geometry for one frame.
///
/// This type owns only bounds and resize geometry. The host retains session state, active
/// identity, interaction semantics, and scene composition.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SessionSidebarLayout {
    sessions_bounds: Option<Rect>,
    main_bounds: Rect,
    sash_track: Option<Rect>,
    resize_snapshot: Option<SplitViewResizeSnapshot>,
}

impl SessionSidebarLayout {
    /// Returns the optional Sessions Part bounds.
    pub const fn sessions_bounds(self) -> Option<Rect> {
        self.sessions_bounds
    }

    /// Returns the main Part bounds.
    pub const fn main_bounds(self) -> Rect {
        self.main_bounds
    }

    /// Returns the sash track used to paint and hit-test the Sessions divider.
    pub const fn sash_track(self) -> Option<Rect> {
        self.sash_track
    }

    /// Returns the resize snapshot matching the resolved Sessions sash.
    pub const fn resize_snapshot(self) -> Option<SplitViewResizeSnapshot> {
        self.resize_snapshot
    }
}

#[cfg(test)]
#[path = "session_workspace_tests.rs"]
mod tests;
