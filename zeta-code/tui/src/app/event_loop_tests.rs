use super::activate_pointer_item;
use super::scroll_pointer_item;
use super::update_pointer_hover;
use crate::app::App;
use crate::app::AppCommand;
use crate::app::AppEvent;
use crate::app::command_panel::CommandPanelPointerTarget;
use crate::app::frame;
use crate::app::frame::InputPointerTarget;
use crate::sessions::Event as SessionEvent;
use crate::terminal::mouse::MouseMode;
use crate::thread::Command as ThreadCommand;
use crate::thread::Event as ThreadEvent;
use crate::thread::composer::ChatComposerPointerTarget;
use crate::thread::composer::CompletionView;
use crate::thread::transcript::TranscriptScrollDirection;
use crate::widgets::list_selection::ListSelectionGroup;
use crate::widgets::list_selection::ListSelectionItem;
use crate::widgets::list_selection::ListSelectionItemId;
use crate::widgets::list_selection::ListSelectionModel;
use crate::widgets::search_box::SearchBoxModel;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::layout::Rect;
use zeta_protocol::Session;
use zeta_protocol::SessionId;
use zeta_protocol::SessionManagerInfo;
use zeta_protocol::SessionStatus;
use zeta_protocol::SessionThread;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadStatus;

#[test]
fn enhanced_panel_wheel_scrolls_only_content_and_preserves_search_focus() {
    let mut app = App::new();
    app.update(AppEvent::HelpOpened(
        ListSelectionModel::new(
            "Items",
            vec![ListSelectionGroup::new(
                "All",
                (0..40)
                    .map(|index| ListSelectionItem::new(format!("Item {index}")))
                    .collect(),
            )],
        )
        .with_search(SearchBoxModel::new("Search")),
    ));
    let area = Rect::new(0, 0, 80, 20);
    app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    let body_row = (0..area.height)
        .find(|row| {
            matches!(
                frame::input_pointer_target_at(&app, area, 2, *row),
                Some(InputPointerTarget::CommandPanel(
                    CommandPanelPointerTarget::Item(0)
                ))
            )
        })
        .unwrap();
    assert!(!frame::panel_mouse_contains(
        &app,
        area,
        ratatui::layout::Position::new(2, 19)
    ));
    scroll_pointer_item(&mut app, area, 2, body_row, TranscriptScrollDirection::Down);
    let selection = app.list_selection().unwrap();
    assert!(selection.search().unwrap().input_active());
    assert_eq!(selection.selected_visible_index(), Some(0));
    assert_eq!(
        frame::input_pointer_target_at(&app, area, 2, body_row),
        None
    );
    assert_eq!(
        frame::input_pointer_target_at(&app, area, 2, body_row + 1),
        Some(InputPointerTarget::CommandPanel(
            CommandPanelPointerTarget::Item(1)
        ))
    );
    // The search field and hint bar do not scroll the list.
    for row in [body_row - 1, 19] {
        scroll_pointer_item(&mut app, area, 2, row, TranscriptScrollDirection::Down);
    }
    assert_eq!(
        frame::input_pointer_target_at(&app, area, 2, body_row + 1),
        Some(InputPointerTarget::CommandPanel(
            CommandPanelPointerTarget::Item(1)
        ))
    );
    let mut settings = crate::config::TerminalSettings::default();
    settings.set_mouse_interactions(false);
    app.update(crate::config::Event::SettingsReceived(settings));
    scroll_pointer_item(
        &mut app,
        area,
        2,
        body_row + 1,
        TranscriptScrollDirection::Down,
    );
    assert_eq!(
        frame::input_pointer_target_at(&app, area, 2, body_row + 1),
        Some(InputPointerTarget::CommandPanel(
            CommandPanelPointerTarget::Item(1)
        ))
    );
    assert_eq!(app.mouse_mode(), MouseMode::TerminalSelection);
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(
        frame::input_pointer_target_at(&app, area, 2, body_row),
        Some(InputPointerTarget::CommandPanel(
            CommandPanelPointerTarget::Item(0)
        ))
    );
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(app.command_panel().is_none());
    assert_eq!(app.mouse_mode(), MouseMode::TerminalSelection);
}

#[test]
fn pointer_move_tracks_hover_without_changing_the_keyboard_completion() {
    let mut app = App::new();
    app.insert_text("/");
    let area = Rect::new(0, 0, 80, 20);
    let third_completion_row = frame::layout(&app, area).input.y - 4;

    update_pointer_hover(&mut app, area, 2, third_completion_row);
    assert!(matches!(app.completion(), Some(CompletionView::Slash(view)) if view.selected == 0));
    assert!(matches!(
        app.hovered_pointer_target(),
        Some(InputPointerTarget::Composer(
            ChatComposerPointerTarget::CompletionItem(2)
        ))
    ));

    update_pointer_hover(&mut app, area, 1, third_completion_row);
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
                == Some(InputPointerTarget::CommandPanel(
                    CommandPanelPointerTarget::Item(1),
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
        Some(&InputPointerTarget::CommandPanel(
            CommandPanelPointerTarget::Item(1)
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
                == Some(InputPointerTarget::CommandPanel(
                    CommandPanelPointerTarget::Tab(1),
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
fn pointer_click_explicitly_focuses_the_command_panel_search_box() {
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
                == Some(InputPointerTarget::CommandPanel(
                    CommandPanelPointerTarget::Search,
                ))
            {
                target = Some((column, row));
                break 'cells;
            }
        }
    }
    let (column, row) = target.expect("command panel search box should be clickable");

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
    app.update(SessionEvent::CatalogReceived(vec![Session {
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
    app.update(ThreadEvent::ContextChanged {
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
    assert!(matches!(
        activate_pointer_item(&mut app, area, column, row),
        Some(AppCommand::Sessions(
            crate::sessions::Command::Preview { .. }
        ))
    ));
    assert!(app.overlay().is_none());
    assert!(app.session_preview().is_some());
    assert!(app.session_manager_view().is_none());
}

#[test]
fn terminal_owned_mouse_wheel_does_not_scroll_the_tui_transcript() {
    let mut app = App::new();
    for index in 0..12 {
        app.update(ThreadEvent::FailureReported(format!("failure {index}")));
    }
    let area = Rect::new(0, 0, 50, 16);
    let transcript = frame::layout(&app, area).session.transcript;

    assert_eq!(
        scroll_pointer_item(
            &mut app,
            area,
            transcript.x,
            transcript.y,
            TranscriptScrollDirection::Up,
        ),
        None
    );
    assert!(app.transcript_scroll().anchor().is_none());
    app.navigate_transcript(TranscriptScrollDirection::Up, area);
    assert!(app.transcript_scroll().anchor().is_some());
    app.handle_key_in_area(KeyEvent::new(KeyCode::End, KeyModifiers::CONTROL), area);
    assert!(app.transcript_scroll().anchor().is_none());
}
#[test]
fn transcript_keyboard_navigation_still_requests_older_history() {
    let mut app = App::new();
    app.update(ThreadEvent::FailureReported("only loaded message".into()));
    let area = Rect::new(0, 0, 50, 16);

    assert_eq!(
        app.navigate_transcript(TranscriptScrollDirection::Up, area),
        Some(AppCommand::Thread(ThreadCommand::LoadOlderHistory))
    );
    assert!(app.transcript_scroll().anchor().is_some());
}
