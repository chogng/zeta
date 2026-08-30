use super::activate_pointer_item;
use super::schedule_action;
use super::update_pointer_hover;
use crate::app::App;
use crate::app::AppCommand;
use crate::app::AppEvent;
use crate::app::frame;
use crate::app::frame::InputPointerTarget;
use crate::components::chat_composer::ChatComposerPointerTarget;
use crate::components::chat_input::CompletionView;
use crate::components::list_selection::ListSelectionGroup;
use crate::components::list_selection::ListSelectionItem;
use crate::components::list_selection::ListSelectionItemId;
use crate::components::list_selection::ListSelectionModel;
use crate::components::pane::PaneSpec;
use crate::components::search_box::SearchBoxModel;
use crate::mouse::MouseMode;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::layout::Rect;
use std::collections::VecDeque;
use zeta_protocol::Session;
use zeta_protocol::SessionId;
use zeta_protocol::SessionManagerInfo;
use zeta_protocol::SessionStatus;
use zeta_protocol::SessionThread;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadStatus;

#[test]
fn request_actions_wait_for_the_active_request_without_losing_order() {
    let mut queued = VecDeque::new();
    let first = AppCommand::OpenConfigPane;
    let second = AppCommand::OpenRewindPane;
    let third = AppCommand::OpenThemePane;

    assert!(schedule_action(Some(first), true, &mut queued).is_none());
    assert!(schedule_action(Some(second), true, &mut queued).is_none());
    assert!(schedule_action(Some(third), true, &mut queued).is_none());
    assert_eq!(queued.len(), 3);
    assert!(matches!(
        schedule_action(None, false, &mut queued),
        Some(AppCommand::OpenConfigPane)
    ));
    assert!(matches!(
        schedule_action(None, false, &mut queued),
        Some(AppCommand::OpenRewindPane)
    ));
    assert!(matches!(
        schedule_action(None, false, &mut queued),
        Some(AppCommand::OpenThemePane)
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
fn pointer_move_tracks_hover_without_changing_the_keyboard_completion() {
    let mut app = App::new();
    app.insert_text("/");
    let area = Rect::new(0, 0, 80, 20);

    update_pointer_hover(&mut app, area, 2, 12);
    assert!(matches!(app.completion(), Some(CompletionView::Slash(view)) if view.selected == 0));
    assert!(matches!(
        app.hovered_pointer_target(),
        Some(InputPointerTarget::Composer(
            ChatComposerPointerTarget::CompletionItem(2)
        ))
    ));

    update_pointer_hover(&mut app, area, 1, 12);
    assert!(matches!(app.completion(), Some(CompletionView::Slash(view)) if view.selected == 0));
    assert!(app.hovered_pointer_target().is_none());
}

#[test]
fn pointer_move_tracks_a_feature_row_without_changing_its_keyboard_selection() {
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
    )));
    let area = Rect::new(0, 0, 80, 24);
    let mut target = None;
    'rows: for row in area.y..area.bottom() {
        for column in area.x..area.right() {
            if frame::input_pointer_target_at(&app, area, column, row)
                == Some(InputPointerTarget::Composer(
                    ChatComposerPointerTarget::PaneItem(1),
                ))
            {
                target = Some((column, row));
                break 'rows;
            }
        }
    }
    let (column, row) = target.expect("second feature row should be clickable");

    update_pointer_hover(&mut app, area, column, row);

    assert_eq!(
        app.list_selection().unwrap().selected_visible_index(),
        Some(0)
    );
    assert_eq!(
        app.hovered_pointer_target(),
        Some(&InputPointerTarget::Composer(
            ChatComposerPointerTarget::PaneItem(1)
        ))
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
    )));
    let area = Rect::new(0, 0, 80, 24);
    assert_eq!(app.mouse_mode(), MouseMode::TuiCapture);

    let mut target = None;
    'cells: for row in area.y..area.bottom() {
        for column in area.x..area.right() {
            if frame::input_pointer_target_at(&app, area, column, row)
                == Some(InputPointerTarget::Composer(
                    ChatComposerPointerTarget::PaneTab(1),
                ))
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

#[test]
fn pointer_click_explicitly_focuses_the_pane_search_box() {
    let mut app = App::new();
    app.update(AppEvent::ListSelectionPaneOpened(PaneSpec::new(
        ListSelectionModel::new(
            "Feature",
            vec![ListSelectionGroup::new(
                "Items",
                vec![ListSelectionItem::new("Searchable")],
            )],
        )
        .with_search(SearchBoxModel::new("Search features")),
    )));
    let area = Rect::new(0, 0, 80, 24);
    let mut target = None;
    'cells: for row in area.y..area.bottom() {
        for column in area.x..area.right() {
            if frame::input_pointer_target_at(&app, area, column, row)
                == Some(InputPointerTarget::Composer(
                    ChatComposerPointerTarget::PaneSearch,
                ))
            {
                target = Some((column, row));
                break 'cells;
            }
        }
    }
    let (column, row) = target.expect("pane search box should be clickable");

    update_pointer_hover(&mut app, area, column, row);
    assert_eq!(app.list_selection().unwrap().query(), "");
    assert_eq!(activate_pointer_item(&mut app, area, column, row), None);
    app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));

    assert_eq!(app.list_selection().unwrap().query(), "s");
}

#[test]
fn pointer_hover_does_not_focus_manager_and_click_opens_the_target_preview() {
    let mut app = App::new();
    let session_id = SessionId::new("pointer-session").unwrap();
    let thread_id = ThreadId::new("pointer-thread").unwrap();
    app.update(AppEvent::SessionCatalogReceived(vec![Session {
        session_id: session_id.clone(),
        title: "Pointer session".into(),
        status: SessionStatus::Active,
        manager: SessionManagerInfo::default(),
        threads: vec![SessionThread {
            thread_id: thread_id.clone(),
            title: "Pointer thread".into(),
            created_at_unix_ms: 0,
            completed_turn_duration_ms: 0,
            active_turn_started_at_unix_ms: None,
            usage: Default::default(),
            parent_thread_id: None,
            forked_from_id: None,
            status: ThreadStatus::Active,
        }],
    }]));
    app.update(AppEvent::ThreadContextChanged {
        session_id,
        thread_id,
    });
    app.insert_text("/sessions");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let area = Rect::new(0, 0, 80, 24);
    let mut session_cell = None;
    'cells: for row in area.y..area.bottom() {
        for column in area.x..area.right() {
            if matches!(
                frame::input_pointer_target_at(&app, area, column, row),
                Some(InputPointerTarget::SessionManager(ref target)) if target.is_session()
            ) {
                update_pointer_hover(&mut app, area, column, row);
                session_cell = Some((column, row));
                break 'cells;
            }
        }
    }
    let (column, row) = session_cell.expect("the Session row should be interactive");

    assert!(!app.session_manager_focused());
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        None
    );
    assert!(app.session_manager_view().is_none());

    app.insert_text("/sessions");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(activate_pointer_item(&mut app, area, column, row), None);
    assert_eq!(app.quick_view().unwrap().title(), "Session preview");
    assert!(app.session_manager_view().is_some());
}
