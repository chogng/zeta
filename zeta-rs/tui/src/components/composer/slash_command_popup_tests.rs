use super::SlashCommandPopup;
use crate::components::composer::SlashCommand;
use crate::components::composer::SlashCommandItem;
use crate::components::composer::slash_commands::SlashCommandRegistry;

fn sync(popup: &mut SlashCommandPopup, input: &str) {
    popup.sync_input(input, input.len(), &SlashCommandRegistry::default());
}

#[test]
fn bare_slash_exposes_every_registered_command() {
    let mut popup = SlashCommandPopup::default();

    sync(&mut popup, "/");

    let expected = super::super::slash_commands::built_in_slash_commands()
        .into_iter()
        .map(|(_, command)| SlashCommandItem::Builtin(command))
        .collect::<Vec<_>>();
    assert_eq!(popup.view().unwrap().commands, expected);
}

#[test]
fn query_filters_commands_and_input_changes_reopen_a_dismissed_popup() {
    let mut popup = SlashCommandPopup::default();
    sync(&mut popup, "/");
    popup.dismiss();
    assert_eq!(popup.view(), None);

    sync(&mut popup, "/q");

    assert_eq!(
        popup.view().unwrap().commands,
        &[SlashCommandItem::Builtin(SlashCommand::Quit)]
    );
}

#[test]
fn selection_wraps_in_both_directions() {
    let mut popup = SlashCommandPopup::default();
    sync(&mut popup, "/");

    popup.select_previous();
    assert_eq!(
        popup.selected_command(),
        Some(SlashCommandItem::Builtin(SlashCommand::Exit))
    );

    popup.select_next();
    assert_eq!(
        popup.selected_command(),
        Some(SlashCommandItem::Builtin(SlashCommand::Status))
    );
}

#[test]
fn command_lookup_returns_only_a_visible_registered_command() {
    let mut popup = SlashCommandPopup::default();
    sync(&mut popup, "/m");

    assert_eq!(
        popup.command_at(0),
        Some(SlashCommandItem::Builtin(SlashCommand::Mcp))
    );
    assert_eq!(
        popup.command_at(1),
        Some(SlashCommandItem::Builtin(SlashCommand::Model))
    );
    assert_eq!(popup.command_at(2), None);

    popup.dismiss();
    assert_eq!(popup.command_at(0), None);
}

#[test]
fn cursor_after_the_command_name_or_non_slash_input_closes_the_popup() {
    let registry = SlashCommandRegistry::default();
    let mut popup = SlashCommandPopup::default();
    sync(&mut popup, "/");

    popup.sync_input("/quit details", "/quit ".len(), &registry);
    assert_eq!(popup.view(), None);

    sync(&mut popup, "hello");
    assert_eq!(popup.view(), None);
}

#[test]
fn moving_the_cursor_back_into_the_name_reopens_discovery_without_losing_the_tail() {
    let registry = SlashCommandRegistry::default();
    let mut popup = SlashCommandPopup::default();

    popup.sync_input("/res details", 4, &registry);

    assert_eq!(
        popup.selected_command(),
        Some(SlashCommandItem::Builtin(SlashCommand::Resume))
    );
}

#[test]
fn unmatched_query_keeps_an_empty_popup_visible() {
    let mut popup = SlashCommandPopup::default();

    sync(&mut popup, "/unknown");

    assert!(popup.view().unwrap().commands.is_empty());
    assert_eq!(popup.selected_command(), None);
}
