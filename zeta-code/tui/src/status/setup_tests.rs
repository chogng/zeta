use super::list_selection;
use crate::render::test_context;
use crate::status::StatusLineItem;
use crate::status::StatusLineSelectionAction;
use crate::status::StatusLineSettings;
use crate::widgets::list_selection::ListSelectionState;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use unicode_width::UnicodeWidthStr;

#[test]
fn setup_lists_each_item_with_a_description_checkbox_and_toggle_action() {
    let mut settings = StatusLineSettings::default();
    settings.set(StatusLineItem::GitChanges, false);
    let view = list_selection(&settings, 7);
    let state = ListSelectionState::new(view.model);

    assert_eq!(state.title(), "Status line");
    assert!(!state.show_tabs());
    assert_eq!(
        state
            .visible_items()
            .iter()
            .map(|item| (item.label(), item.description().unwrap().trim()))
            .collect::<Vec<_>>(),
        vec![
            ("Permissions", "Current permission mode [ ✔ ]"),
            ("Model", "Configured model [ ✔ ]"),
            (
                "Cache hit rate",
                "Cached input as a share of total input [   ]"
            ),
            (
                "Reference cost",
                "Current Thread accumulated reference cost [   ]"
            ),
            ("Git branch", "Current Git branch [ ✔ ]"),
            ("Git changes", "Working tree changes [   ]"),
        ]
    );
    assert!(matches!(
        view.actions
            .get(state.visible_items()[5].id().unwrap())
            .unwrap(),
        StatusLineSelectionAction::SetEnabled(edit)
            if edit.expected_revision == 7
                && edit.item == StatusLineItem::GitChanges
                && edit.enabled
    ));
}

#[test]
fn setup_aligns_items_descriptions_and_checkboxes_in_three_columns() {
    let mut settings = StatusLineSettings::default();
    settings.set(StatusLineItem::GitChanges, false);
    let view = list_selection(&settings, 1);
    let state = ListSelectionState::new(view.model);
    let backend = TestBackend::new(80, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            crate::widgets::list_selection::draw(frame, frame.area(), &state, test_context())
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let rows = (0..10)
        .map(|row| {
            (0..80)
                .map(|column| buffer[(column, row)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>();
    let permissions = rows.iter().find(|row| row.contains("Permissions")).unwrap();
    let model = rows
        .iter()
        .find(|row| row.contains("Configured model"))
        .unwrap();
    let git_branch = rows
        .iter()
        .find(|row| row.contains("Current Git branch"))
        .unwrap();
    let git_changes = rows
        .iter()
        .find(|row| row.contains("Working tree changes"))
        .unwrap();
    let cache_hit_rate = rows
        .iter()
        .find(|row| row.contains("Cached input as a share"))
        .unwrap();
    let reference_cost = rows
        .iter()
        .find(|row| row.contains("accumulated reference cost"))
        .unwrap();

    let description_column = column_of(permissions, "Current permission mode");
    assert_eq!(column_of(permissions, "Permissions"), 2);
    assert_eq!(column_of(model, "Model"), 2);
    assert_eq!(column_of(cache_hit_rate, "Cache hit rate"), 2);
    assert_eq!(column_of(reference_cost, "Reference cost"), 2);
    assert!(permissions.starts_with("> "));
    assert_eq!(column_of(model, "Configured model"), description_column);
    assert_eq!(
        column_of(git_branch, "Current Git branch"),
        description_column
    );
    assert_eq!(
        column_of(cache_hit_rate, "Cached input as a share"),
        description_column
    );
    assert_eq!(
        column_of(reference_cost, "Current Thread accumulated reference cost"),
        description_column
    );
    assert_eq!(
        column_of(git_changes, "Working tree changes"),
        description_column
    );
    let checkbox_column = column_of(model, "[ ✔ ]");
    assert_eq!(column_of(permissions, "[ ✔ ]"), checkbox_column);
    assert_eq!(column_of(git_branch, "[ ✔ ]"), checkbox_column);
    assert_eq!(column_of(cache_hit_rate, "[   ]"), checkbox_column);
    assert_eq!(column_of(reference_cost, "[   ]"), checkbox_column);
    assert_eq!(column_of(git_changes, "[   ]"), checkbox_column);
    insta::assert_snapshot!("status_line_settings_with_accounting", rows.join("\n"));
}

fn column_of(row: &str, text: &str) -> usize {
    row[..row.find(text).unwrap()].width()
}
