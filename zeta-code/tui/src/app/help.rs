use crate::components::composer::built_in_slash_commands;
use crate::components::pane::PaneViewModel;
use crate::components::search_box::SearchBoxModel;
use crate::components::selection::SelectionItem;
use crate::components::selection::SelectionTab;
use crate::components::selection::SelectionViewModel;

pub(crate) fn help_selection_view() -> PaneViewModel<SelectionViewModel> {
    let commands = built_in_slash_commands()
        .into_iter()
        .map(|(name, command)| {
            SelectionItem::new(format!("/{name}")).with_description(command.description())
        })
        .collect();
    PaneViewModel::new(
        SelectionViewModel::new("Help", vec![SelectionTab::new("Commands", commands)])
            .without_tab_bar()
            .with_search(SearchBoxModel::new("Search commands")),
        "Space search  ·  ↑/↓ select  ·  Esc back",
    )
}
