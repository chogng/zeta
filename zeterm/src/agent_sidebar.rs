use crate::NativeApp;
use crate::shell_interaction::AGENT_SIDEBAR_RESIZE_HANDLE;
use crate::shell_scene::agent_sidebar_resize_snapshot_for_viewport;
use zeta_ui::Point;
use zeta_ui::SplitViewPane;
use zeta_ui::SplitViewResizeSnapshot;
use zeta_winit::ElementState;
use zui::DispatchInvalidation;

const DEFAULT_WIDTH: f32 = 320.0;
const MINIMUM_WIDTH: f32 = 240.0;
const MAXIMUM_WIDTH: f32 = 560.0;
const MINIMUM_MAIN_WIDTH: f32 = 240.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum AgentSidebarVisibility {
    #[default]
    Collapsed,
    Expanded,
}

/// Runtime visibility and layout state for the Agent sidebar container.
///
/// Files and SCM content are owned by `zeta_agent_sidebar::AgentSidebar`; this
/// type only controls whether their shared host participates in shell layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AgentSidebarState {
    visibility: AgentSidebarVisibility,
    preferred_width: f32,
    resize: Option<AgentSidebarResize>,
}

impl Default for AgentSidebarState {
    fn default() -> Self {
        Self {
            visibility: AgentSidebarVisibility::Collapsed,
            preferred_width: DEFAULT_WIDTH,
            resize: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct AgentSidebarResize {
    pointer_origin: f32,
    snapshot: SplitViewResizeSnapshot,
    current_size: f32,
}

impl AgentSidebarState {
    #[cfg(test)]
    pub(crate) const fn expanded() -> Self {
        Self {
            visibility: AgentSidebarVisibility::Expanded,
            preferred_width: DEFAULT_WIDTH,
            resize: None,
        }
    }

    pub(crate) const fn is_expanded(self) -> bool {
        matches!(self.visibility, AgentSidebarVisibility::Expanded)
    }

    pub(crate) const fn is_resizing(self) -> bool {
        self.resize.is_some()
    }

    pub(crate) fn toggle(&mut self) {
        self.visibility = match self.visibility {
            AgentSidebarVisibility::Collapsed => AgentSidebarVisibility::Expanded,
            AgentSidebarVisibility::Expanded => AgentSidebarVisibility::Collapsed,
        };
        self.resize = None;
    }

    pub(crate) fn expand(&mut self) {
        self.visibility = AgentSidebarVisibility::Expanded;
    }

    pub(crate) fn is_visible_for(self, available_width: f32) -> bool {
        self.is_expanded() && available_width >= MINIMUM_WIDTH + MINIMUM_MAIN_WIDTH
    }

    pub(crate) const fn preferred_width(self) -> f32 {
        self.preferred_width
    }

    pub(crate) const fn minimum_main_width(self) -> f32 {
        MINIMUM_MAIN_WIDTH
    }

    pub(crate) fn pane_sizing(self, available_width: f32) -> SplitViewPane {
        let sidebar = SplitViewPane::new(self.preferred_width, MINIMUM_WIDTH, MAXIMUM_WIDTH);
        if self.is_visible_for(available_width) {
            sidebar
        } else {
            sidebar.hidden()
        }
    }

    pub(crate) fn start_resizing(
        &mut self,
        snapshot: SplitViewResizeSnapshot,
        pointer_x: f32,
    ) -> bool {
        if self.resize.is_some() {
            return false;
        }
        let current_size = snapshot.resize(0.0).next_size();
        self.resize = Some(AgentSidebarResize {
            pointer_origin: pointer_x,
            snapshot,
            current_size,
        });
        true
    }

    pub(crate) fn resize_to(&mut self, pointer_x: f32) -> bool {
        let Some(mut resize) = self.resize else {
            return false;
        };
        let next = resize.snapshot.resize(pointer_x - resize.pointer_origin);
        debug_assert_eq!(next.previous_index(), 0);
        debug_assert_eq!(next.next_index(), 1);
        if next.next_size() == resize.current_size {
            return false;
        }
        resize.current_size = next.next_size();
        self.resize = Some(resize);
        self.preferred_width = next.next_size();
        true
    }

    pub(crate) fn finish_resizing(&mut self) -> bool {
        if self.resize.is_none() {
            return false;
        }
        self.resize = None;
        true
    }
}

impl NativeApp {
    pub(super) fn route_agent_sidebar_resize_move(&mut self, point: Point) -> bool {
        if !self.agent_sidebar.is_resizing() {
            return false;
        }
        if self.agent_sidebar.resize_to(point.x) {
            self.terminal_selection.clear();
            self.rebuild_presentation();
            self.request_redraw();
        }
        self.update_cursor();
        true
    }

    pub(super) fn route_agent_sidebar_resize_button(&mut self, state: ElementState) -> bool {
        match state {
            ElementState::Pressed => {
                let Some(point) = self.cursor_position else {
                    return false;
                };
                let over_handle = self.presentation.as_ref().is_some_and(|presentation| {
                    presentation.interaction_frame().target_at(point)
                        == Some(AGENT_SIDEBAR_RESIZE_HANDLE)
                });
                let Some(snapshot) = agent_sidebar_resize_snapshot_for_viewport(
                    self.logical_viewport(),
                    self.session_sidebar,
                    self.agent_sidebar,
                ) else {
                    return false;
                };
                if !over_handle || !self.agent_sidebar.start_resizing(snapshot, point.x) {
                    return false;
                }
            }
            ElementState::Released => {
                if !self.agent_sidebar.finish_resizing() {
                    return false;
                }
            }
        }
        self.rebuild_presentation();
        let hover_changed = self
            .cursor_position
            .zip(self.presentation.as_ref())
            .is_some_and(|(point, presentation)| {
                self.ui_dispatch
                    .pointer_moved(point, presentation.interaction_frame())
                    .invalidation
                    == DispatchInvalidation::Paint
            });
        if hover_changed {
            self.rebuild_presentation();
        }
        self.update_cursor();
        self.request_redraw();
        true
    }

    pub(super) fn cancel_agent_sidebar_resize(&mut self) {
        if self.agent_sidebar.finish_resizing() {
            self.rebuild_presentation();
            self.update_cursor();
            self.request_redraw();
        }
    }
}

#[cfg(test)]
#[path = "agent_sidebar_tests.rs"]
mod tests;
