use super::activate_pointer_item;
use super::schedule_action;
use super::select_hovered_popup_item;
use crate::app::App;
use crate::app::AppCommand;
use crate::app::AppEvent;
use crate::app::frame;
use crate::components::chat_input::SuggestView;
use crate::components::chat_input_area::ChatInputAreaPointerTarget;
use crate::components::list_selection::ListSelectionGroup;
use crate::components::list_selection::ListSelectionItem;
use crate::components::list_selection::ListSelectionItemId;
use crate::components::list_selection::ListSelectionModel;
use crate::components::pane::PaneSpec;
use crate::mouse::MouseMode;
use ratatui::layout::Rect;
use std::collections::VecDeque;

#[test]
fn request_actions_wait_for_the_active_request_without_losing_order() {
    let mut queued = VecDeque::new();
    let first = AppCommand::OpenConfigPane;
    let second = AppCommand::OpenRewindPane;

    assert!(schedule_action(Some(first), true, &mut queued).is_none());
    assert!(schedule_action(Some(second), true, &mut queued).is_none());
    assert_eq!(queued.len(), 2);
    assert!(matches!(
        schedule_action(None, false, &mut queued),
        Some(AppCommand::OpenConfigPane)
    ));
    assert!(matches!(
        schedule_action(None, false, &mut queued),
        Some(AppCommand::OpenRewindPane)
    ));
}

#[test]
fn quit_bypasses_a_pending_request() {
    let mut queued = VecDeque::new();

    assert!(matches!(
        schedule_action(Some(AppCommand::Quit), true, &mut queued),
        Some(AppCommand::Quit)
    ));
    assert!(queued.is_empty());
}

#[test]
fn pointer_move_selects_the_hovered_popup_row_and_preserves_it_outside() {
    let mut app = App::new();
    app.insert_text("/");
    let area = Rect::new(0, 0, 80, 20);

    select_hovered_popup_item(&mut app, area, 2, 12);
    assert!(matches!(app.suggest(), Some(SuggestView::Slash(view)) if view.selected == 2));

    select_hovered_popup_item(&mut app, area, 1, 12);
    assert!(matches!(app.suggest(), Some(SuggestView::Slash(view)) if view.selected == 2));
}

#[test]
fn pointer_move_selects_an_actionable_row_in_a_generic_feature_pane() {
    let mut app = App::new();
    app.update(AppEvent::ListSelectionPaneOpened(PaneSpec::new(
        ListSelectionModel::new(
            "Feature",
            vec![ListSelectionGroup::new(
                "Items",
                vec![
                    ListSelectionItem::new("First").with_id(ListSelectionItemId::new("first")),
                    ListSelectionItem::new("Second").with_id(ListSelectionItemId::new("second")),
                ],
            )],
        )
        .without_tab_bar(),
        "Esc back",
    )));
    let area = Rect::new(0, 0, 80, 24);
    let mut target = None;
    'rows: for row in area.y..area.bottom() {
        for column in area.x..area.right() {
            if frame::input_pointer_target_at(&app, area, column, row)
                == Some(ChatInputAreaPointerTarget::PaneItem(1))
            {
                target = Some((column, row));
                break 'rows;
            }
        }
    }
    let (column, row) = target.expect("second feature row should be clickable");

    select_hovered_popup_item(&mut app, area, column, row);

    assert_eq!(
        app.list_selection().unwrap().selected_visible_index(),
        Some(1)
    );
}

#[test]
fn pointer_click_switches_a_selection_tab() {
    let mut app = App::new();
    app.update(AppEvent::ListSelectionPaneOpened(PaneSpec::new(
        ListSelectionModel::new(
            "Feature",
            vec![
                ListSelectionGroup::new("First", vec![ListSelectionItem::new("Read only")]),
                ListSelectionGroup::new("Second", vec![ListSelectionItem::new("Another item")]),
            ],
        ),
        "Esc back",
    )));
    let area = Rect::new(0, 0, 80, 24);
    assert_eq!(app.mouse_mode(), MouseMode::UiClick);

    let mut target = None;
    'cells: for row in area.y..area.bottom() {
        for column in area.x..area.right() {
            if frame::input_pointer_target_at(&app, area, column, row)
                == Some(ChatInputAreaPointerTarget::PaneTab(1))
            {
                target = Some((column, row));
                break 'cells;
            }
        }
    }
    let (column, row) = target.expect("second selection tab should be clickable");

    assert_eq!(activate_pointer_item(&mut app, area, column, row), None);
    assert_eq!(app.list_selection().unwrap().active_tab().label(), "Second");
}
