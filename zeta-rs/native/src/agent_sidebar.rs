use zeta_ui::SplitViewPane;

const DEFAULT_WIDTH: f32 = 320.0;
const MINIMUM_MAIN_WIDTH: f32 = 240.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum AgentSidebarVisibility {
    #[default]
    Collapsed,
    Expanded,
}

/// Runtime visibility and layout state for the Agent sidebar container.
///
/// Explorer and editor content are owned by `AgentSidebarWorkspace`; this type
/// only controls whether their shared host participates in shell layout.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct AgentSidebarState {
    visibility: AgentSidebarVisibility,
}

impl AgentSidebarState {
    #[cfg(test)]
    pub(crate) const fn expanded() -> Self {
        Self {
            visibility: AgentSidebarVisibility::Expanded,
        }
    }

    pub(crate) const fn is_expanded(self) -> bool {
        matches!(self.visibility, AgentSidebarVisibility::Expanded)
    }

    pub(crate) fn toggle(&mut self) {
        self.visibility = match self.visibility {
            AgentSidebarVisibility::Collapsed => AgentSidebarVisibility::Expanded,
            AgentSidebarVisibility::Expanded => AgentSidebarVisibility::Collapsed,
        };
    }

    pub(crate) fn is_visible_for(self, available_width: f32) -> bool {
        self.is_expanded() && available_width >= DEFAULT_WIDTH + MINIMUM_MAIN_WIDTH
    }

    pub(crate) const fn preferred_width(self) -> f32 {
        DEFAULT_WIDTH
    }

    pub(crate) const fn minimum_main_width(self) -> f32 {
        MINIMUM_MAIN_WIDTH
    }

    pub(crate) fn pane_sizing(self, available_width: f32) -> SplitViewPane {
        let sidebar = SplitViewPane::new(DEFAULT_WIDTH, DEFAULT_WIDTH, DEFAULT_WIDTH);
        if self.is_visible_for(available_width) {
            sidebar
        } else {
            sidebar.hidden()
        }
    }
}

#[cfg(test)]
#[path = "agent_sidebar_tests.rs"]
mod tests;
