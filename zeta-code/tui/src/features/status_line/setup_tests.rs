use super::selection_view;
use crate::components::selection::SelectionViewState;
use crate::features::status_line::StatusLineItem;
use crate::features::status_line::StatusLineSelectionAction;
use crate::features::status_line::StatusLineSettings;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use unicode_width::UnicodeWidthStr;

#[test]
fn setup_lists_each_item_with_a_description_boolean_and_toggle_action() {
    let mut settings = StatusLineSettings::default();
    settings.set(StatusLineItem::GitChanges, false);
    let view = selection_view(settings, 7);
    let state = SelectionViewState::new(view.model.into_body());

    assert_eq!(state.title(), "Status line");
    assert!(!state.show_tabs());
    assert_eq!(
        state
            .visible_items()
            .iter()
            .map(|item| (item.label(), item.description().unwrap().trim()))
            .collect::<Vec<_>>(),
        vec![
            ("Permissions", "Current permission mode true"),
            ("Model", "Configured model true"),
            ("Git branch", "Current Git branch true"),
            ("Git changes", "Working tree changes false"),
        ]
    );
    assert!(matches!(
        view.actions
            .get(state.visible_items()[3].id().unwrap())
            .unwrap(),
        StatusLineSelectionAction::SetEnabled(edit)
            if edit.expected_revision == 7
                && edit.item == StatusLineItem::GitChanges
                && edit.enabled
    ));
}

#[test]
fn setup_aligns_items_descriptions_and_booleans_in_three_columns() {
    let mut settings = StatusLineSettings::default();
    settings.set(StatusLineItem::GitChanges, false);
    let view = selection_view(settings, 1);
    let state = SelectionViewState::new(view.model.into_body());
    let backend = TestBackend::new(80, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| crate::components::selection::draw(frame, frame.area(), &state))
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

    let description_column = column_of(permissions, "Current permission mode");
    assert_eq!(column_of(model, "Configured model"), description_column);
    assert_eq!(
        column_of(git_branch, "Current Git branch"),
        description_column
    );
    assert_eq!(
        column_of(git_changes, "Working tree changes"),
        description_column
    );
    let boolean_column = column_of(model, "true");
    assert_eq!(column_of(permissions, "true"), boolean_column);
    assert_eq!(column_of(git_branch, "true"), boolean_column);
    assert_eq!(column_of(git_changes, "false"), boolean_column);
}

fn column_of(row: &str, text: &str) -> usize {
    row[..row.find(text).unwrap()].width()
}
