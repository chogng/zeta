use super::SelectionActivationMode;
use super::SelectionInputOutcome;
use super::SelectionItem;
use super::SelectionItemId;
use super::SelectionTab;
use super::SelectionViewModel;
use super::SelectionViewState;
use super::tab_row_count;
use crate::components::search_box::SearchBoxModel;
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
        .with_search(SearchBoxModel::new("Search help")),
    )
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn active_tab_label(state: &SelectionViewState) -> &str {
    state.tabs()[state.active_tab_index()].label()
}

#[test]
fn arrow_keys_switch_tabs_and_wrap() {
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

    state.handle_key(key(KeyCode::Char(' ')));
    for character in "esc".chars() {
        state.handle_key(key(KeyCode::Char(character)));
    }
    state.handle_key(key(KeyCode::Right));

    assert_eq!(state.query(), "esc");
    assert_eq!(state.visible_items().len(), 2);
    assert_eq!(state.visible_items()[0].label(), "Esc");
    assert_eq!(state.visible_items()[1].label(), "↑ / ↓");
}

#[test]
fn filtering_and_navigation_keep_selection_in_visible_range() {
    let mut state = state();

    state.handle_key(key(KeyCode::Down));
    assert_eq!(state.selected_visible_index(), Some(1));
    state.handle_key(key(KeyCode::Char(' ')));
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
fn candidate_ranking_uses_match_quality_and_preserves_equal_model_order() {
    let mut state = SelectionViewState::new(
        SelectionViewModel::new(
            "Help",
            vec![SelectionTab::new(
                "Commands",
                vec![
                    SelectionItem::new("show status"),
                    SelectionItem::new("s-t-a-t-u-s"),
                    SelectionItem::new("status"),
                    SelectionItem::new("status line"),
                    SelectionItem::new("appstatus"),
                    SelectionItem::new("second description").with_description("status"),
                    SelectionItem::new("first description").with_description("status"),
                ],
            )],
        )
        .with_search(SearchBoxModel::new("Search commands"))
        .with_initial_selected(4),
    );

    state.handle_key(key(KeyCode::Char(' ')));
    for character in "status".chars() {
        state.handle_key(key(KeyCode::Char(character)));
    }

    assert_eq!(
        state
            .visible_items()
            .into_iter()
            .map(SelectionItem::label)
            .collect::<Vec<_>>(),
        vec![
            "status",
            "status line",
            "show status",
            "appstatus",
            "s-t-a-t-u-s",
            "second description",
            "first description",
        ]
    );
    assert_eq!(state.selected_visible_index(), Some(0));
    assert_eq!(state.selected_item().unwrap().label(), "status");
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
fn enter_and_space_activate_actionable_items() {
    let item_id = SelectionItemId::new("toggle-skill");
    let mut actionable = SelectionViewState::new(
        SelectionViewModel::new(
            "Skills",
            vec![SelectionTab::new(
                "All",
                vec![SelectionItem::new("review").with_id(item_id.clone())],
            )],
        )
        .with_activation_mode(SelectionActivationMode::EnterOrSpace),
    );

    assert_eq!(
        actionable.handle_key(key(KeyCode::Enter)),
        SelectionInputOutcome::Activate(item_id.clone())
    );
    assert_eq!(
        actionable.handle_key(key(KeyCode::Char(' '))),
        SelectionInputOutcome::Activate(item_id)
    );
}

#[test]
fn space_enters_search_before_becoming_search_text() {
    let mut read_only = state();

    assert!(read_only.search().is_some());
    assert!(!read_only.search_active());
    assert_eq!(
        read_only.handle_key(key(KeyCode::Char(' '))),
        SelectionInputOutcome::Consumed
    );
    assert!(read_only.search_active());
    assert_eq!(read_only.query(), "");
    read_only.handle_key(key(KeyCode::Char(' ')));
    assert_eq!(read_only.query(), " ");
}

#[test]
fn enter_only_actions_keep_space_available_for_search() {
    let mut state = SelectionViewState::new(
        SelectionViewModel::new(
            "Themes",
            vec![SelectionTab::new(
                "All",
                vec![SelectionItem::new("Zeta Code Dark").with_id(SelectionItemId::new("theme"))],
            )],
        )
        .with_search(SearchBoxModel::new("Search themes")),
    );

    state.handle_key(key(KeyCode::Char(' ')));
    for character in "zeta code".chars() {
        state.handle_key(key(KeyCode::Char(character)));
    }

    assert_eq!(state.query(), "zeta code");
    assert_eq!(state.visible_items()[0].label(), "Zeta Code Dark");
}

#[test]
fn selection_without_search_ignores_text_and_space() {
    let mut state = SelectionViewState::new(SelectionViewModel::new(
        "Themes",
        vec![SelectionTab::new(
            "Themes",
            vec![SelectionItem::new("Dark mode")],
        )],
    ));

    state.handle_key(key(KeyCode::Char(' ')));
    state.handle_key(key(KeyCode::Char('d')));

    assert!(state.search().is_none());
    assert!(!state.search_active());
    assert_eq!(state.query(), "");
}

#[test]
fn escape_closes_search_before_dismissing_the_view() {
    let mut state = state();
    state.handle_key(key(KeyCode::Char(' ')));
    state.handle_key(key(KeyCode::Char('s')));

    assert_eq!(
        state.handle_key(key(KeyCode::Esc)),
        SelectionInputOutcome::Consumed
    );
    assert!(!state.search_active());
    assert_eq!(state.query(), "");
    assert_eq!(
        state.handle_key(key(KeyCode::Esc)),
        SelectionInputOutcome::Dismiss
    );
}

#[test]
fn paste_only_filters_after_space_enters_search_mode() {
    let mut state = state();

    state.handle_paste("status".into());
    assert_eq!(state.query(), "");
    state.handle_key(key(KeyCode::Char(' ')));
    state.handle_paste("status".into());

    assert_eq!(state.query(), "status");
    assert_eq!(state.visible_items()[0].label(), "/status");
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
