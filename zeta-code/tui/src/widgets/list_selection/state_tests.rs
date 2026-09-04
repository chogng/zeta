use super::ListSelectionActivationMode;
use super::ListSelectionAdjustment;
use super::ListSelectionGroup;
use super::ListSelectionInputOutcome;
use super::ListSelectionItem;
use super::ListSelectionItemId;
use super::ListSelectionModel;
use super::ListSelectionState;
use crate::widgets::search_box::SearchBoxModel;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::layout::Rect;

fn state() -> ListSelectionState {
    ListSelectionState::new(
        ListSelectionModel::new(
            "Help",
            vec![
                ListSelectionGroup::new(
                    "Commands",
                    vec![
                        ListSelectionItem::new("/status").with_description("show status"),
                        ListSelectionItem::new("/model").with_description("show model"),
                    ],
                ),
                ListSelectionGroup::new(
                    "Keys",
                    vec![
                        ListSelectionItem::new("↑ / ↓").with_description("move selection"),
                        ListSelectionItem::new("Esc").with_description("close"),
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

fn active_tab_label(state: &ListSelectionState) -> &str {
    state.active_tab().label()
}

#[test]
fn actionable_rows_expose_pointer_hit_testing_and_activation() {
    let first_id = ListSelectionItemId::new("first");
    let second_id = ListSelectionItemId::new("second");
    let mut state = ListSelectionState::new(
        ListSelectionModel::new(
            "Items",
            vec![ListSelectionGroup::new(
                "All",
                vec![
                    ListSelectionItem::new("First").with_id(first_id),
                    ListSelectionItem::new("Second").with_id(second_id.clone()),
                ],
            )],
        )
        .without_tab_bar(),
    );
    let area = Rect::new(0, 0, 80, 10);

    assert_eq!(state.item_index_at(area, 0, 0), Some(0));
    assert_eq!(state.item_index_at(area, 2, 1), Some(1));
    assert_eq!(state.item_index_at(area, 78, 1), None);
    assert_eq!(state.activate_visible_item(1), Some(second_id));
    assert_eq!(state.selected_visible_index(), Some(0));
}

#[test]
fn read_only_rows_cannot_be_activated() {
    let mut state = ListSelectionState::new(ListSelectionModel::new(
        "Status",
        vec![ListSelectionGroup::new(
            "Details",
            vec![ListSelectionItem::new("Read only")],
        )],
    ));

    assert!(!state.select_visible_item(0));
    assert_eq!(state.activate_visible_item(0), None);
}

#[test]
fn mouse_click_switches_tabs() {
    let mut state = state();
    let area = Rect::new(0, 0, 80, 10);

    assert_eq!(state.tab_index_at(area, 14, 0), Some(1));
    assert!(state.select_tab(1));
    assert_eq!(active_tab_label(&state), "Keys");
    assert_eq!(state.selected_visible_index(), Some(0));
}

#[test]
fn search_hit_testing_and_explicit_focus_share_the_search_geometry() {
    let mut state = state();
    let area = Rect::new(0, 0, 80, 10);

    assert!(state.search_contains(area, 2, 1));
    assert!(!state.search_contains(area, 1, 1));
    assert!(state.focus_search());
    state.handle_key(key(KeyCode::Char('m')));

    assert_eq!(state.query(), "m");
}

#[test]
fn tab_keys_switch_tabs_and_wrap() {
    let mut state = state();

    state.handle_key(key(KeyCode::Tab));
    assert_eq!(active_tab_label(&state), "Keys");
    state.handle_key(key(KeyCode::Tab));
    assert_eq!(active_tab_label(&state), "Commands");
    state.handle_key(key(KeyCode::BackTab));
    assert_eq!(active_tab_label(&state), "Keys");
}

#[test]
fn arrow_keys_adjust_the_selected_actionable_item() {
    let item_id = ListSelectionItemId::new("follow-up-mode");
    let mut state = ListSelectionState::new(
        ListSelectionModel::new(
            "Config",
            vec![ListSelectionGroup::new(
                "Config",
                vec![ListSelectionItem::new("Follow-up messages").with_id(item_id.clone())],
            )],
        )
        .without_tab_bar(),
    );

    assert_eq!(
        state.handle_key(key(KeyCode::Left)),
        ListSelectionInputOutcome::Adjust(item_id.clone(), ListSelectionAdjustment::Previous)
    );
    assert_eq!(
        state.handle_key(key(KeyCode::Right)),
        ListSelectionInputOutcome::Adjust(item_id, ListSelectionAdjustment::Next)
    );
}

#[test]
fn tab_switching_preserves_the_search_query() {
    let mut state = state();

    state.handle_key(key(KeyCode::Up));
    for character in "esc".chars() {
        state.handle_key(key(KeyCode::Char(character)));
    }
    state.handle_key(key(KeyCode::Tab));

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
    state.handle_key(key(KeyCode::Home));
    state.handle_key(key(KeyCode::Up));
    for character in "status".chars() {
        state.handle_key(key(KeyCode::Char(character)));
    }

    assert_eq!(state.visible_items().len(), 1);
    assert_eq!(state.selected_visible_index(), Some(0));
    for _ in 0.."status".len() {
        state.handle_key(key(KeyCode::Backspace));
    }
    state.handle_key(key(KeyCode::Down));
    state.handle_key(key(KeyCode::Down));
    assert_eq!(state.selected_visible_index(), Some(1));
}

#[test]
fn candidate_ranking_uses_match_quality_and_preserves_equal_model_order() {
    let mut state = ListSelectionState::new(
        ListSelectionModel::new(
            "Help",
            vec![ListSelectionGroup::new(
                "Commands",
                vec![
                    ListSelectionItem::new("show status"),
                    ListSelectionItem::new("s-t-a-t-u-s"),
                    ListSelectionItem::new("status"),
                    ListSelectionItem::new("status line"),
                    ListSelectionItem::new("appstatus"),
                    ListSelectionItem::new("second description").with_description("status"),
                    ListSelectionItem::new("first description").with_description("status"),
                ],
            )],
        )
        .with_search(SearchBoxModel::new("Search commands"))
        .with_initial_selected(4),
    );

    state.handle_key(key(KeyCode::Home));
    state.handle_key(key(KeyCode::Up));
    for character in "status".chars() {
        state.handle_key(key(KeyCode::Char(character)));
    }

    assert_eq!(
        state
            .visible_items()
            .into_iter()
            .map(ListSelectionItem::label)
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
        ListSelectionInputOutcome::Dismiss
    );
}

#[test]
fn control_c_also_dismisses_the_active_view() {
    let mut state = state();

    assert_eq!(
        state.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
        ListSelectionInputOutcome::Dismiss
    );
}

#[test]
fn enter_and_space_activate_actionable_items() {
    let item_id = ListSelectionItemId::new("toggle-skill");
    let mut actionable = ListSelectionState::new(
        ListSelectionModel::new(
            "Skills",
            vec![ListSelectionGroup::new(
                "All",
                vec![ListSelectionItem::new("review").with_id(item_id.clone())],
            )],
        )
        .with_activation_mode(ListSelectionActivationMode::EnterOrSpace),
    );

    assert_eq!(
        actionable.handle_key(key(KeyCode::Enter)),
        ListSelectionInputOutcome::Activate(item_id.clone())
    );
    assert_eq!(
        actionable.handle_key(key(KeyCode::Char(' '))),
        ListSelectionInputOutcome::Activate(item_id)
    );
}

#[test]
fn up_moves_focus_from_the_first_item_to_search_and_then_tabs() {
    let mut read_only = state();

    assert!(read_only.search().is_some());
    assert!(read_only.items_focused());
    assert_eq!(
        read_only.handle_key(key(KeyCode::Up)),
        ListSelectionInputOutcome::Consumed
    );
    assert!(read_only.search_focused());
    assert_eq!(read_only.query(), "");
    read_only.handle_key(key(KeyCode::Char(' ')));
    assert_eq!(read_only.query(), " ");
    read_only.handle_key(key(KeyCode::Up));
    assert!(read_only.tabs_focused());
    read_only.handle_key(key(KeyCode::Down));
    assert!(read_only.search_focused());
    read_only.handle_key(key(KeyCode::Down));
    assert!(read_only.items_focused());
}

#[test]
fn enter_only_actions_keep_space_available_for_search() {
    let mut state = ListSelectionState::new(
        ListSelectionModel::new(
            "Themes",
            vec![ListSelectionGroup::new(
                "All",
                vec![
                    ListSelectionItem::new("Zeta Code Dark")
                        .with_id(ListSelectionItemId::new("theme")),
                ],
            )],
        )
        .with_search(SearchBoxModel::new("Search themes")),
    );

    state.handle_key(key(KeyCode::Up));
    for character in "zeta code".chars() {
        state.handle_key(key(KeyCode::Char(character)));
    }

    assert_eq!(state.query(), "zeta code");
    assert_eq!(state.visible_items()[0].label(), "Zeta Code Dark");
}

#[test]
fn selection_without_search_ignores_text_and_space() {
    let mut state = ListSelectionState::new(ListSelectionModel::new(
        "Themes",
        vec![ListSelectionGroup::new(
            "Themes",
            vec![ListSelectionItem::new("Dark mode")],
        )],
    ));

    state.handle_key(key(KeyCode::Char(' ')));
    state.handle_key(key(KeyCode::Char('d')));

    assert!(state.search().is_none());
    assert_eq!(state.query(), "");
}

#[test]
fn escape_dismisses_the_view_while_search_is_active() {
    let mut state = state();
    state.handle_key(key(KeyCode::Up));
    state.handle_key(key(KeyCode::Char('s')));

    assert_eq!(
        state.handle_key(key(KeyCode::Esc)),
        ListSelectionInputOutcome::Dismiss
    );
    assert!(state.search_focused());
    assert_eq!(state.query(), "s");
}

#[test]
fn paste_only_filters_while_search_is_focused() {
    let mut state = state();

    state.handle_paste("status".into());
    assert_eq!(state.query(), "");
    state.handle_key(key(KeyCode::Up));
    state.handle_paste("status".into());

    assert_eq!(state.query(), "status");
    assert_eq!(state.visible_items()[0].label(), "/status");
}

#[test]
fn view_model_can_name_its_empty_state() {
    let state = ListSelectionState::new(
        ListSelectionModel::new("Skills", vec![ListSelectionGroup::new("All", Vec::new())])
            .with_empty_message("No configured skill sources"),
    );

    assert_eq!(state.empty_message(), "No configured skill sources");
    assert!(state.visible_items().is_empty());
}
