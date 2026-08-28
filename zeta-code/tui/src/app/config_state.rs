use super::App;
use super::SelectionActions;
use crate::features::config::ConfigSelectionView;

impl App {
    pub(super) fn show_config_view(&mut self, view: ConfigSelectionView) {
        self.push_selection_view(view.model, SelectionActions::Config(view.actions));
    }

    pub(super) fn replace_config_view(&mut self, view: ConfigSelectionView) {
        self.replace_selection_view(view.model, SelectionActions::Config(view.actions));
    }
}
