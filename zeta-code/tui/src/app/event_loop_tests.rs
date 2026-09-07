use super::activate_pointer_item;
use super::handle_mouse;
use super::scroll_pointer_item;
use super::update_pointer_hover;
use crate::app::App;
use crate::app::AppCommand;
use crate::app::AppEvent;
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
use crate::widgets::list_selection::ListSelectionModel;
use crate::widgets::search_box::SearchBoxModel;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use crossterm::event::MouseButton;
use crossterm::event::MouseEvent;
use crossterm::event::MouseEventKind;
use ratatui::layout::Rect;
use zeta_protocol::Session;
use zeta_protocol::SessionId;
use zeta_protocol::SessionManagerInfo;
use zeta_protocol::SessionStatus;
use zeta_protocol::SessionThread;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadStatus;

#[test]
fn fixed_command_panels_ignore_mouse_and_keep_keyboard_navigation() {
    let mut app = App::new();
    app.update(AppEvent::HelpOpened(
        ListSelectionModel::new(
            "Items",
            vec![
                ListSelectionGroup::new(
                    "First",
                    (0..40)
                        .map(|index| ListSelectionItem::new(format!("Item {index}")))
                        .collect(),
                ),
                ListSelectionGroup::new("Second", vec![ListSelectionItem::new("Other")]),
            ],
        )
        .with_search(SearchBoxModel::new("Search")),
    ));
    for area in [Rect::new(0, 0, 80, 24), Rect::new(0, 0, 12, 3)] {
        assert_terminal_mouse(&mut app, area);
        let selection = app.list_selection().unwrap();
        assert_eq!(selection.active_tab().label(), "First");
        assert_eq!(selection.selected_visible_index(), Some(0));
        assert_eq!(selection.query(), "");
    }
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(
        app.list_selection().unwrap().selected_visible_index(),
        Some(1)
    );
    app.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE));
    assert_eq!(app.list_selection().unwrap().query(), "2");
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(app.command_panel().is_none());
}

fn assert_terminal_mouse(app: &mut App, area: Rect) {
    assert_eq!(app.mouse_mode(), MouseMode::TerminalSelection);
    for row in area.y..area.bottom() {
        for column in area.x..area.right() {
            assert!(!frame::overlay_mouse_contains(
                app,
                area,
                ratatui::layout::Position::new(column, row)
            ));
            assert_eq!(frame::input_pointer_target_at(app, area, column, row), None);
            for kind in [
                MouseEventKind::Moved,
                MouseEventKind::Down(MouseButton::Left),
                MouseEventKind::Drag(MouseButton::Left),
                MouseEventKind::Up(MouseButton::Left),
                MouseEventKind::ScrollUp,
                MouseEventKind::ScrollDown,
            ] {
                assert_eq!(
                    handle_mouse(
                        app,
                        area,
                        MouseEvent {
                            kind,
                            column,
                            row,
                            modifiers: KeyModifiers::NONE,
                        }
                    ),
                    None
                );
            }
            assert_eq!(activate_pointer_item(app, area, column, row), None);
        }
    }
    assert!(app.hovered_pointer_target().is_none());
    assert!(app.pressed_pointer_target().is_none());
    assert!(app.screen_selection().range().is_none());
}

#[test]
fn completion_click_is_disabled_with_enhancement_and_keyboard_still_works() {
    let mut app = App::new();
    app.insert_text("/q");
    let area = Rect::new(0, 0, 80, 24);
    let target = (0..area.height)
        .flat_map(|row| (0..area.width).map(move |column| (column, row)))
        .find(|(column, row)| frame::input_pointer_target_at(&app, area, *column, *row).is_some())
        .expect("completion is clickable");
    let mut settings = crate::config::TerminalSettings::default();
    settings.set_mouse_interactions(false);
    app.update(crate::config::Event::SettingsReceived(settings));
    assert_terminal_mouse(&mut app, area);
    assert_eq!(app.input(), "/q");
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some(AppCommand::Quit)
    );

    let mut app = App::new();
    app.insert_text("/q");
    assert_eq!(
        activate_pointer_item(&mut app, area, target.0, target.1),
        Some(AppCommand::Quit)
    );
}

#[test]
fn detail_overlay_captures_only_its_surface_and_releases_mouse_on_close() {
    let mut app = App::new();
    let area = Rect::new(0, 0, 80, 24);
    let before = frame::layout(&app, area).session.composer;
    app.show_overlay(crate::widgets::detail_list::DetailList::new(
        "Output",
        vec![crate::widgets::detail_list::DetailListRow::new(
            "stdout", "details",
        )],
    ));
    assert_eq!(frame::layout(&app, area).session.composer, before);
    assert_eq!(app.mouse_mode(), MouseMode::TuiCapture);
    let surface = app
        .overlay()
        .unwrap()
        .surface(frame::transient_area(&app, area));
    for row in 0..area.height {
        for column in 0..area.width {
            let position = ratatui::layout::Position::new(column, row);
            assert_eq!(
                frame::overlay_mouse_contains(&app, area, position),
                surface.contains(position)
            );
            assert_eq!(activate_pointer_item(&mut app, area, column, row), None);
        }
    }
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_terminal_mouse(&mut app, area);
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
fn fixed_session_manager_ignores_mouse_without_changing_focus_or_opening_preview() {
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
    app.insert_text("keep this draft");
    assert_terminal_mouse(&mut app, area);
    assert!(!app.session_manager_focused());
    assert!(app.session_preview().is_none());
    assert!(app.session_manager_view().is_some());
    assert_eq!(app.input(), "keep this draft");
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

#[test]
fn disabling_enhancement_during_a_drag_ignores_queued_mouse_events() {
    let mut app = App::new();
    app.insert_text("/");
    let area = Rect::new(0, 0, 80, 24);
    let row = frame::layout(&app, area).input.y - 1;
    let event = |kind, column| MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    };
    handle_mouse(&mut app, area, event(MouseEventKind::Moved, 2));
    handle_mouse(
        &mut app,
        area,
        event(MouseEventKind::Down(MouseButton::Left), 2),
    );
    assert!(app.hovered_pointer_target().is_some());
    assert!(app.pressed_pointer_target().is_some());
    handle_mouse(
        &mut app,
        area,
        event(MouseEventKind::Drag(MouseButton::Left), 8),
    );
    assert!(app.screen_selection().range().is_some());
    let mut settings = crate::config::TerminalSettings::default();
    settings.set_mouse_interactions(false);
    app.update(crate::config::Event::SettingsReceived(settings));
    assert_eq!(
        handle_mouse(
            &mut app,
            area,
            event(MouseEventKind::Up(MouseButton::Left), 8)
        ),
        None
    );
    assert_terminal_mouse(&mut app, area);
    assert_eq!(app.input(), "/");
}

// Run under a PTY so TerminalSession emits the actual mouse-mode protocol.
#[test]
#[ignore = "requires a PTY with a nonzero window size"]
fn real_terminal_mouse_handoff() {
    let mut terminal = crate::terminal::TerminalSession::open().unwrap();
    let area = terminal.area().unwrap();
    assert!(area.width >= 40 && area.height >= 12);
    let mut app = App::new();
    app.update(AppEvent::HelpOpened(ListSelectionModel::new(
        "Help",
        vec![ListSelectionGroup::new(
            "Commands",
            vec![ListSelectionItem::new("Help")],
        )],
    )));
    super::draw_terminal(&mut terminal, &mut app).unwrap();
    assert_terminal_mouse(&mut app, area);
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    app.insert_text("/q");
    super::draw_terminal(&mut terminal, &mut app).unwrap();
    let (column, row) = (0..area.height)
        .flat_map(|row| (0..area.width).map(move |column| (column, row)))
        .find(|(column, row)| frame::input_pointer_target_at(&app, area, *column, *row).is_some())
        .unwrap();
    let event = |kind| MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    };
    handle_mouse(
        &mut app,
        area,
        event(MouseEventKind::Down(MouseButton::Left)),
    );
    let outcome = handle_mouse(&mut app, area, event(MouseEventKind::Up(MouseButton::Left)));
    assert_eq!(
        super::finish_pointer_gesture(&mut app, &terminal, outcome).unwrap(),
        Some(AppCommand::Quit)
    );
    super::draw_terminal(&mut terminal, &mut app).unwrap();
    app.insert_text("/q");
    super::draw_terminal(&mut terminal, &mut app).unwrap();
    let mut settings = crate::config::TerminalSettings::default();
    settings.set_mouse_interactions(false);
    app.update(crate::config::Event::SettingsReceived(settings));
    super::draw_terminal(&mut terminal, &mut app).unwrap();
    assert_terminal_mouse(&mut app, area);
    assert_eq!(app.input(), "/q");
}
