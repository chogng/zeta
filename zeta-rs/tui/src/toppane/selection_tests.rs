use super::SelectionInputOutcome;
use super::SelectionItem;
use super::SelectionItemId;
use super::SelectionTab;
use super::SelectionViewModel;
use super::SelectionViewState;
use super::tab_row_count;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

fn state() -> SelectionViewState {
    SelectionViewState::new(
        SelectionViewModel::new(
            "Help",
            vec![
                SelectionTab::new(
                    "Commands",
                    vec![
                        SelectionItem::new("/status").with_description("show status"),
                        SelectionItem::new("/model").with_description("show model"),
                    ],
                ),
                SelectionTab::new(
                    "Keys",
                    vec![
                        SelectionItem::new("↑ / ↓").with_description("move selection"),
                        SelectionItem::new("Esc").with_description("close"),
                    ],
                ),
            ],
        )
        .with_search_placeholder("Search help"),
    )
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn active_tab_label(state: &SelectionViewState) -> &str {
    state.tabs()[state.active_tab_index()].label()
}

#[test]
fn left_and_right_switch_tabs_and_wrap() {
    let mut state = state();

    state.handle_key(key(KeyCode::Right));
    assert_eq!(active_tab_label(&state), "Keys");
    state.handle_key(key(KeyCode::Right));
    assert_eq!(active_tab_label(&state), "Commands");
    state.handle_key(key(KeyCode::Left));
    assert_eq!(active_tab_label(&state), "Keys");
}

#[test]
fn tab_switching_preserves_the_search_query() {
    let mut state = state();

    for character in "esc".chars() {
        state.handle_key(key(KeyCode::Char(character)));
    }
    state.handle_key(key(KeyCode::Right));

    assert_eq!(state.query(), "esc");
    assert_eq!(state.visible_items().len(), 1);
    assert_eq!(state.visible_items()[0].label(), "Esc");
}

#[test]
fn filtering_and_navigation_keep_selection_in_visible_range() {
    let mut state = state();

    state.handle_key(key(KeyCode::Down));
    assert_eq!(state.selected_visible_index(), Some(1));
    for character in "status".chars() {
        state.handle_key(key(KeyCode::Char(character)));
    }

    assert_eq!(state.visible_items().len(), 1);
    assert_eq!(state.selected_visible_index(), Some(0));
    for _ in 0.."status".len() {
        state.handle_key(key(KeyCode::Backspace));
    }
    state.handle_key(key(KeyCode::Up));
    assert_eq!(state.selected_visible_index(), Some(1));
}

#[test]
fn escape_requests_view_dismissal() {
    let mut state = state();

    assert_eq!(
        state.handle_key(key(KeyCode::Esc)),
        SelectionInputOutcome::Dismiss
    );
}

#[test]
fn control_c_also_dismisses_the_active_view() {
    let mut state = state();

    assert_eq!(
        state.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
        SelectionInputOutcome::Dismiss
    );
}

#[test]
fn space_activates_an_actionable_item_and_remains_search_text_for_read_only_items() {
    let item_id = SelectionItemId::new("toggle-skill");
    let mut actionable = SelectionViewState::new(SelectionViewModel::new(
        "Skills",
        vec![SelectionTab::new(
            "All",
            vec![SelectionItem::new("review").with_id(item_id.clone())],
        )],
    ));

    assert_eq!(
        actionable.handle_key(key(KeyCode::Char(' '))),
        SelectionInputOutcome::Activate(item_id)
    );

    let mut read_only = state();
    assert_eq!(
        read_only.handle_key(key(KeyCode::Char(' '))),
        SelectionInputOutcome::Consumed
    );
    assert_eq!(read_only.query(), " ");
}

#[test]
fn narrow_width_wraps_tabs_without_hiding_them() {
    let state = state();

    assert_eq!(tab_row_count(state.tabs(), 80), 1);
    assert_eq!(tab_row_count(state.tabs(), 12), 2);
}

#[test]
fn view_model_can_name_its_empty_state() {
    let state = SelectionViewState::new(
        SelectionViewModel::new("Skills", vec![SelectionTab::new("All", Vec::new())])
            .with_empty_message("No configured skill sources"),
    );

    assert_eq!(state.empty_message(), "No configured skill sources");
    assert!(state.visible_items().is_empty());
}
