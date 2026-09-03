use super::Completion;
use super::activate_pointer_item;
use super::schedule_action;
use super::update_pointer_hover;
use crate::app::App;
use crate::app::AppCommand;
use crate::app::AppEvent;
use crate::app::frame;
use crate::app::frame::InputPointerTarget;
use crate::app::input_surface::InputSurfacePointerTarget;
use crate::app::requests::RequestLane;
use crate::app::requests::RequestTasks;
use crate::terminal::mouse::MouseMode;
use crate::thread::composer::ChatComposerPointerTarget;
use crate::thread::composer::CompletionView;
use crate::widgets::list_selection::ListSelectionGroup;
use crate::widgets::list_selection::ListSelectionItem;
use crate::widgets::list_selection::ListSelectionItemId;
use crate::widgets::list_selection::ListSelectionModel;
use crate::widgets::search_box::SearchBoxModel;
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
fn control_and_read_actions_bypass_a_busy_write_lane_without_losing_write_order() {
    let mut app = App::new();
    let mut requests = RequestTasks::default();
    let (release, wait) = std::sync::mpsc::sync_channel(0);
    requests.spawn(
        Some(RequestLane::Write),
        "zeta-tui-test-write",
        move || {
            wait.recv().expect("the test releases the write request");
            Completion::Presentation(Err("finished".into()))
        },
        &mut app,
    );
    let mut queued = VecDeque::new();
    let write = AppCommand::SetTheme {
        preference: "zeta-code-dark".into(),
    };

    assert!(schedule_action(Some(write), &requests, &mut queued).is_none());
    assert!(matches!(
        schedule_action(Some(AppCommand::OpenConfigEditor), &requests, &mut queued),
        Some(AppCommand::OpenConfigEditor)
    ));
    assert!(matches!(
        schedule_action(Some(AppCommand::Interrupt), &requests, &mut queued),
        Some(AppCommand::Interrupt)
    ));
    assert_eq!(queued.len(), 1);
    release
        .send(())
        .expect("the write request remains alive until released");
    let completed = (0..10_000)
        .find_map(|_| {
            let completed = requests.poll();
            if completed.is_empty() {
                std::thread::yield_now();
                None
            } else {
                Some(completed)
            }
        })
        .expect("the released write request completes");
    assert_eq!(completed.len(), 1);
    assert!(matches!(
        schedule_action(None, &requests, &mut queued),
        Some(AppCommand::SetTheme { .. })
    ));
}

#[test]
fn quit_bypasses_a_pending_request() {
    let mut app = App::new();
    let mut requests = RequestTasks::default();
    requests.spawn(
        Some(RequestLane::Write),
        "zeta-tui-test-write",
        || Completion::Presentation(Err("finished".into())),
        &mut app,
    );
    let mut queued = VecDeque::new();

    assert!(matches!(
        schedule_action(Some(AppCommand::Quit), &requests, &mut queued),
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
    app.update(AppEvent::HelpOpened(
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
    ));
    let area = Rect::new(0, 0, 80, 24);
    let mut target = None;
    'rows: for row in area.y..area.bottom() {
        for column in area.x..area.right() {
            if frame::input_pointer_target_at(&app, area, column, row)
                == Some(InputPointerTarget::InputSurface(
                    InputSurfacePointerTarget::Item(1),
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
        Some(&InputPointerTarget::InputSurface(
            InputSurfacePointerTarget::Item(1)
        ))
    );
}

#[test]
fn pointer_click_switches_a_selection_tab() {
    let mut app = App::new();
    app.update(AppEvent::HelpOpened(ListSelectionModel::new(
        "Feature",
        vec![
            ListSelectionGroup::new("First", vec![ListSelectionItem::new("Read only")]),
            ListSelectionGroup::new("Second", vec![ListSelectionItem::new("Another item")]),
        ],
    )));
    let area = Rect::new(0, 0, 80, 24);
    assert_eq!(app.mouse_mode(), MouseMode::TuiCapture);

    let mut target = None;
    'cells: for row in area.y..area.bottom() {
        for column in area.x..area.right() {
            if frame::input_pointer_target_at(&app, area, column, row)
                == Some(InputPointerTarget::InputSurface(
                    InputSurfacePointerTarget::Tab(1),
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
fn pointer_click_explicitly_focuses_the_composer_search_box() {
    let mut app = App::new();
    app.update(AppEvent::HelpOpened(
        ListSelectionModel::new(
            "Feature",
            vec![ListSelectionGroup::new(
                "Items",
                vec![ListSelectionItem::new("Searchable")],
            )],
        )
        .with_search(SearchBoxModel::new("Search features")),
    ));
    let area = Rect::new(0, 0, 80, 24);
    let mut target = None;
    'cells: for row in area.y..area.bottom() {
        for column in area.x..area.right() {
            if frame::input_pointer_target_at(&app, area, column, row)
                == Some(InputPointerTarget::InputSurface(
                    InputSurfacePointerTarget::Search,
                ))
            {
                target = Some((column, row));
                break 'cells;
            }
        }
    }
    let (column, row) = target.expect("composer search box should be clickable");

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
                Some(InputPointerTarget::SessionManager(_))
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
    assert_eq!(app.overlay().unwrap().title(), "Session preview");
    assert!(app.session_manager_view().is_some());
}
