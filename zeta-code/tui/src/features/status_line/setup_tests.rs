use super::selection_view;
use crate::components::selection::SelectionViewState;
use crate::features::status_line::StatusLineItem;
use crate::features::status_line::StatusLineSelectionAction;
use crate::features::status_line::StatusLineSettings;

#[test]
fn setup_lists_each_item_with_a_boolean_and_toggle_action() {
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
            ("Permissions", "true"),
            ("Model", "true"),
            ("Git branch", "true"),
            ("Git changes", "false"),
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
