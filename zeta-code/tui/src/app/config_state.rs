use super::App;
use super::SelectionActions;
use crate::features::config::ConfigSelectionView;

impl App {
    pub(super) fn show_config_view(&mut self, view: ConfigSelectionView) {
        self.interaction_pane.show_selection_view(view.model);
        self.selection_actions
            .push(SelectionActions::Config(view.actions));
    }

    pub(super) fn replace_config_view(&mut self, view: ConfigSelectionView) {
        self.interaction_pane.replace_selection_view(view.model);
        match self.selection_actions.last_mut() {
            Some(actions) => *actions = SelectionActions::Config(view.actions),
            None => self
                .selection_actions
                .push(SelectionActions::Config(view.actions)),
        }
    }
}
