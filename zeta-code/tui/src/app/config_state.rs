use super::App;
use super::PaneActions;
use crate::features::config::ConfigPaneSpec;

impl App {
    pub(super) fn show_config_pane(&mut self, pane_spec: ConfigPaneSpec) {
        self.push_list_selection_pane(pane_spec.model, PaneActions::Config(pane_spec.actions));
    }

    pub(super) fn replace_config_pane(&mut self, pane_spec: ConfigPaneSpec) {
        self.replace_list_selection_pane(pane_spec.model, PaneActions::Config(pane_spec.actions));
    }
}
