use crate::components::composer::built_in_slash_commands;
use crate::components::selection::SelectionItem;
use crate::components::selection::SelectionTab;
use crate::components::selection::SelectionViewModel;

pub(crate) fn help_selection_view() -> SelectionViewModel {
    let commands = built_in_slash_commands()
        .into_iter()
        .map(|(name, command)| {
            SelectionItem::new(format!("/{name}")).with_description(command.description())
        })
        .collect();
    let keys = [
        ("Enter", "submit the current prompt"),
        ("Ctrl-V", "attach an image from the system clipboard"),
        ("Esc", "close the active view or exit while idle"),
        ("Ctrl-C", "interrupt an active turn or exit while idle"),
        ("← / →", "switch tabs in an interactive view"),
        ("↑ / ↓", "move through visible choices"),
    ]
    .into_iter()
    .map(|(key, description)| SelectionItem::new(key).with_description(description))
    .collect();
    SelectionViewModel::new(
        "Help",
        vec![
            SelectionTab::new("Commands", commands),
            SelectionTab::new("Keys", keys),
        ],
    )
    .with_search_placeholder("Search commands and shortcuts")
}
