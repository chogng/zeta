use super::keymap::app_keybinding_help_items;
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
    let keys = [
        ("Enter", "submit the current prompt"),
        ("Shift-Enter", "insert a newline in the current prompt"),
        ("Ctrl-Home", "load 50 older turns and move to history start"),
        ("Esc", "close the active view"),
        ("← / →", "switch tabs in an interactive view"),
        ("↑ / ↓", "move through visible choices"),
    ]
    .into_iter()
    .chain(app_keybinding_help_items())
    .map(|(key, description)| SelectionItem::new(key).with_description(description))
    .collect();
    PaneViewModel::new(
        SelectionViewModel::new(
            "Help",
            vec![
                SelectionTab::new("Commands", commands),
                SelectionTab::new("Keys", keys),
            ],
        )
        .with_search(SearchBoxModel::new("Search commands and shortcuts")),
        "Space search  ·  ←/→ tabs  ·  ↑/↓ select  ·  Esc back",
    )
}
