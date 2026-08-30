use crate::components::chat_input::built_in_slash_commands;
use crate::components::list_selection::ListSelectionGroup;
use crate::components::list_selection::ListSelectionItem;
use crate::components::list_selection::ListSelectionModel;
use crate::components::pane::PaneSpec;
use crate::components::search_box::SearchBoxModel;

pub(crate) fn help_pane_spec() -> PaneSpec<ListSelectionModel> {
    let commands = built_in_slash_commands()
        .into_iter()
        .map(|(name, command)| {
            ListSelectionItem::new(format!("/{name}")).with_description(command.description())
        })
        .collect();
    PaneSpec::new(
        ListSelectionModel::new("Help", vec![ListSelectionGroup::new("Commands", commands)])
            .without_tab_bar()
            .with_search(SearchBoxModel::new("Search commands")),
    )
}
