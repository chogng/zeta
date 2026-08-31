use crate::components::chat_input::SlashCommandCatalog;
use crate::components::list_selection::ListSelectionGroup;
use crate::components::list_selection::ListSelectionItem;
use crate::components::list_selection::ListSelectionModel;
use crate::components::search_box::SearchBoxModel;

pub(crate) fn help_choices(slash_commands: &SlashCommandCatalog) -> ListSelectionModel {
    let commands = slash_commands
        .commands()
        .iter()
        .map(|command| {
            ListSelectionItem::new(format!("/{}", command.name))
                .with_description(&command.description)
        })
        .collect();
    ListSelectionModel::new("Help", vec![ListSelectionGroup::new("Commands", commands)])
        .without_tab_bar()
        .with_search(SearchBoxModel::new("Search commands"))
}
