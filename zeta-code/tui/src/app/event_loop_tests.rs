use super::activate_pointer_item;
use super::handle_mouse;
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
        assert_tui_capture_without_pointer_actions(&mut app, area);
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

fn assert_tui_capture_without_pointer_actions(app: &mut App, area: Rect) {
    assert_eq!(app.mouse_mode(), MouseMode::TuiCapture);
    for row in area.y..area.bottom() {
        for column in area.x..area.right() {
            assert!(!frame::overlay_mouse_contains(
                app,
                area,
                ratatui::layout::Position::new(column, row)
            ));
            assert_eq!(frame::input_pointer_target_at(app, area, column, row), None);
            assert_eq!(activate_pointer_item(app, area, column, row), None);
        }
    }
}

fn assert_scroll_only_mouse(app: &mut App) {
    assert_eq!(app.mouse_mode(), MouseMode::TuiScroll);
    app.clear_mouse_interaction();
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
    assert_scroll_only_mouse(&mut app);
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
    assert_tui_capture_without_pointer_actions(&mut app, area);
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
    assert_tui_capture_without_pointer_actions(&mut app, area);
    assert!(!app.session_manager_focused());
    assert!(app.session_preview().is_none());
    assert!(app.session_manager_view().is_some());
    assert_eq!(app.input(), "keep this draft");
}

#[test]
fn mouse_wheel_scrolls_the_full_screen_transcript() {
    let mut app = App::new();
    for index in 0..12 {
        app.update(ThreadEvent::FailureReported(format!("failure {index}")));
    }
    let area = Rect::new(0, 0, 50, 16);
    let transcript = frame::layout(&app, area).session.transcript;

    assert!(matches!(
        handle_mouse(
            &mut app,
            area,
            MouseEvent {
                kind: MouseEventKind::ScrollUp,
                column: transcript.x,
                row: transcript.y,
                modifiers: KeyModifiers::NONE,
            }
        ),
        super::MouseAction::Command(None)
    ));
    assert!(app.transcript_scroll().anchor().is_some());
    app.handle_key_in_area(KeyEvent::new(KeyCode::End, KeyModifiers::CONTROL), area);
    assert!(app.transcript_scroll().anchor().is_none());

    app.update(AppEvent::HelpOpened(ListSelectionModel::new(
        "Help",
        vec![ListSelectionGroup::new(
            "Commands",
            vec![ListSelectionItem::new("Help")],
        )],
    )));
    let panel = frame::layout(&app, area).session.composer;
    assert!(matches!(
        handle_mouse(
            &mut app,
            area,
            MouseEvent {
                kind: MouseEventKind::ScrollUp,
                column: panel.x,
                row: panel.y,
                modifiers: KeyModifiers::NONE,
            }
        ),
        super::MouseAction::Command(None)
    ));
    assert!(app.transcript_scroll().anchor().is_none());
}

#[test]
fn scroll_only_mouse_mode_still_scrolls_the_transcript() {
    let mut app = App::new();
    for index in 0..12 {
        app.update(ThreadEvent::FailureReported(format!("failure {index}")));
    }
    let mut settings = crate::config::TerminalSettings::default();
    settings.set_mouse_interactions(false);
    app.update(crate::config::Event::SettingsReceived(settings));
    let area = Rect::new(0, 0, 50, 16);
    let transcript = frame::layout(&app, area).session.transcript;

    assert!(matches!(
        handle_mouse(
            &mut app,
            area,
            MouseEvent {
                kind: MouseEventKind::ScrollUp,
                column: transcript.x,
                row: transcript.y,
                modifiers: KeyModifiers::NONE,
            }
        ),
        super::MouseAction::Command(None)
    ));
    assert!(app.transcript_scroll().anchor().is_some());
    assert_scroll_only_mouse(&mut app);
}

#[test]
fn text_selection_remains_available_without_enhanced_pointer_actions() {
    let mut app = App::new();
    let mut settings = crate::config::TerminalSettings::default();
    settings.set_mouse_interactions(false);
    app.update(crate::config::Event::SettingsReceived(settings));
    let area = Rect::new(0, 0, 50, 16);
    let event = |kind, column| MouseEvent {
        kind,
        column,
        row: 1,
        modifiers: KeyModifiers::NONE,
    };

    handle_mouse(
        &mut app,
        area,
        event(MouseEventKind::Down(MouseButton::Left), 2),
    );
    handle_mouse(
        &mut app,
        area,
        event(MouseEventKind::Drag(MouseButton::Left), 8),
    );
    let outcome = handle_mouse(
        &mut app,
        area,
        event(MouseEventKind::Up(MouseButton::Left), 8),
    );

    assert!(matches!(
        outcome,
        super::MouseAction::Selection(Some(
            crate::terminal::screen_selection::ScreenSelectionOutcome::Selection(_)
        ))
    ));
    assert!(app.hovered_pointer_target().is_none());
    assert!(app.pressed_pointer_target().is_none());
}

#[test]
fn overlays_block_background_wheel_and_selection_until_dismissed() {
    for detail in [false, true] {
        let mut app = App::new();
        let area = Rect::new(0, 0, 80, 24);
        for index in 0..30 {
            app.update(ThreadEvent::FailureReported(format!("failure {index}")));
        }
        app.handle_key_in_area(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE), area);
        let anchor = app.transcript_scroll().anchor().cloned();
        if detail {
            app.show_overlay(crate::widgets::detail_list::DetailList::new(
                "Output",
                vec![crate::widgets::detail_list::DetailListRow::new(
                    "stdout", "details",
                )],
            ));
        } else {
            app.insert_text("/q");
            assert!(frame::completion_visible(&app));
        }
        let transcript = frame::layout(&app, area).session.transcript;
        let outside = (transcript.y..transcript.bottom())
            .map(|row| ratatui::layout::Position::new(transcript.x, row))
            .find(|position| !frame::overlay_mouse_contains(&app, area, *position))
            .unwrap();
        for kind in [
            MouseEventKind::ScrollUp,
            MouseEventKind::ScrollDown,
            MouseEventKind::Down(MouseButton::Left),
            MouseEventKind::Drag(MouseButton::Left),
            MouseEventKind::Up(MouseButton::Left),
        ] {
            assert!(matches!(
                handle_mouse(
                    &mut app,
                    area,
                    MouseEvent {
                        kind,
                        column: outside.x,
                        row: outside.y,
                        modifiers: KeyModifiers::NONE,
                    }
                ),
                super::MouseAction::Selection(None)
            ));
            assert!(app.screen_selection().range().is_none());
            assert!(app.pressed_pointer_target().is_none());
            assert_eq!(app.transcript_scroll().anchor(), anchor.as_ref());
        }
        let inside = (0..area.height)
            .flat_map(|row| {
                (0..area.width).map(move |column| ratatui::layout::Position::new(column, row))
            })
            .find(|position| frame::overlay_mouse_contains(&app, area, *position))
            .unwrap();
        handle_mouse(
            &mut app,
            area,
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: inside.x,
                row: inside.y,
                modifiers: KeyModifiers::NONE,
            },
        );
        handle_mouse(
            &mut app,
            area,
            MouseEvent {
                kind: MouseEventKind::Drag(MouseButton::Left),
                column: outside.x,
                row: outside.y,
                modifiers: KeyModifiers::NONE,
            },
        );
        assert!(app.screen_selection().range().is_none());
        assert!(matches!(
            handle_mouse(
                &mut app,
                area,
                MouseEvent {
                    kind: MouseEventKind::Up(MouseButton::Left),
                    column: inside.x,
                    row: inside.y,
                    modifiers: KeyModifiers::NONE
                }
            ),
            super::MouseAction::Selection(None)
        ));
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        handle_mouse(
            &mut app,
            area,
            MouseEvent {
                kind: MouseEventKind::ScrollDown,
                column: outside.x,
                row: outside.y,
                modifiers: KeyModifiers::NONE,
            },
        );
        assert_ne!(app.transcript_scroll().anchor(), anchor.as_ref());
    }
}

#[test]
fn detail_overlay_still_scrolls_its_own_content_with_the_mouse() {
    let mut app = App::new();
    let area = Rect::new(0, 0, 80, 24);
    app.show_overlay(crate::widgets::detail_list::DetailList::new(
        "Output",
        vec![crate::widgets::detail_list::DetailListRow::new(
            "stdout",
            (0..40)
                .map(|index| format!("line {index:02}"))
                .collect::<Vec<_>>()
                .join("\n"),
        )],
    ));
    let surface = app
        .overlay()
        .unwrap()
        .surface(frame::transient_area(&app, area));
    let render = |app: &App| {
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(area.width, area.height))
                .unwrap();
        terminal.draw(|frame| frame::draw(frame, app)).unwrap();
        terminal.backend().to_string()
    };
    let before = render(&app);
    handle_mouse(
        &mut app,
        area,
        MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: surface.x,
            row: surface.y + 1,
            modifiers: KeyModifiers::NONE,
        },
    );
    assert_ne!(render(&app), before);
    assert!(app.transcript_scroll().anchor().is_none());
}

#[test]
fn issue_manager_blocks_background_transcript_scroll_and_clicks() {
    let mut app = App::new();
    let area = Rect::new(0, 0, 80, 24);
    for index in 0..30 {
        app.update(ThreadEvent::FailureReported(format!("failure {index}")));
    }
    app.handle_key_in_area(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE), area);
    let anchor = app.transcript_scroll().anchor().cloned();
    assert!(anchor.is_some());
    app.insert_text("/issue");
    assert!(matches!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some(AppCommand::Issues(_))
    ));
    assert!(app.issue_manager().is_some());
    let content = frame::layout(&app, area).session.transcript;
    handle_mouse(
        &mut app,
        area,
        MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: content.x,
            row: content.y,
            modifiers: KeyModifiers::NONE,
        },
    );
    for column in content.x..content.right() {
        assert!(frame::input_pointer_target_at(&app, area, column, content.bottom() - 1).is_none());
        activate_pointer_item(&mut app, area, column, content.bottom() - 1);
    }
    assert_eq!(app.transcript_scroll().anchor(), anchor.as_ref());
}

#[test]
fn jump_control_click_and_keyboard_restore_latest_without_changing_the_draft() {
    for width in [16, 50] {
        let mut app = App::new();
        for index in 0..12 {
            app.update(ThreadEvent::FailureReported(format!("failure {index}")));
        }
        app.insert_text("keep this draft");
        let area = Rect::new(0, 0, width, 16);
        app.handle_key_in_area(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE), area);
        let transcript = frame::layout(&app, area).session.transcript;
        let row = transcript.bottom() - 1;
        let columns = (0..width)
            .filter(|column| {
                frame::input_pointer_target_at(&app, area, *column, row)
                    == Some(InputPointerTarget::TranscriptJumpToBottom)
            })
            .collect::<Vec<_>>();
        assert!(!columns.is_empty());
        for column in [columns[0], *columns.last().unwrap()] {
            app.handle_key_in_area(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE), area);
            let event = |kind| MouseEvent {
                kind,
                column,
                row,
                modifiers: KeyModifiers::NONE,
            };
            handle_mouse(&mut app, area, event(MouseEventKind::Moved));
            assert_eq!(
                app.hovered_pointer_target(),
                Some(&InputPointerTarget::TranscriptJumpToBottom)
            );
            handle_mouse(
                &mut app,
                area,
                event(MouseEventKind::Down(MouseButton::Left)),
            );
            assert_eq!(
                app.pressed_pointer_target(),
                Some(&InputPointerTarget::TranscriptJumpToBottom)
            );
            let super::MouseAction::Selection(Some(
                crate::terminal::screen_selection::ScreenSelectionOutcome::Click {
                    position, ..
                },
            )) = handle_mouse(&mut app, area, event(MouseEventKind::Up(MouseButton::Left)))
            else {
                panic!("expected a click");
            };
            activate_pointer_item(&mut app, area, position.x, position.y);
            assert!(app.transcript_scroll().anchor().is_none());
            assert!(frame::input_pointer_target_at(&app, area, column, row).is_none());
            assert_eq!(app.input(), "keep this draft");
        }
        app.handle_key_in_area(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE), area);
        let anchor = app.transcript_scroll().anchor().cloned();
        app.show_overlay(crate::widgets::detail_list::DetailList::new(
            "Output",
            vec![crate::widgets::detail_list::DetailListRow::new(
                "stdout", "details",
            )],
        ));
        activate_pointer_item(&mut app, area, columns[0], row);
        app.handle_key_in_area(KeyEvent::new(KeyCode::End, KeyModifiers::CONTROL), area);
        assert_eq!(app.transcript_scroll().anchor(), anchor.as_ref());
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        let mut settings = crate::config::TerminalSettings::default();
        settings.set_mouse_interactions(false);
        app.update(crate::config::Event::SettingsReceived(settings));
        activate_pointer_item(&mut app, area, columns[0], row);
        assert!(app.transcript_scroll().anchor().is_some());
        app.handle_key_in_area(KeyEvent::new(KeyCode::End, KeyModifiers::CONTROL), area);
        assert!(app.transcript_scroll().anchor().is_none());
    }
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
fn disabling_enhancement_during_a_drag_keeps_the_baseline_selection() {
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
    assert!(matches!(
        handle_mouse(
            &mut app,
            area,
            event(MouseEventKind::Up(MouseButton::Left), 8)
        ),
        super::MouseAction::Selection(Some(
            crate::terminal::screen_selection::ScreenSelectionOutcome::Selection(_)
        ))
    ));
    assert_eq!(app.mouse_mode(), MouseMode::TuiScroll);
    assert!(app.hovered_pointer_target().is_none());
    assert!(app.pressed_pointer_target().is_none());
    assert!(app.screen_selection().range().is_some());
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
    assert_tui_capture_without_pointer_actions(&mut app, area);
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    app.insert_text("/q");
    super::draw_terminal(&mut terminal, &mut app).unwrap();
    let area = terminal.area().unwrap();
    let (column, row) = (area.y..area.bottom())
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
    let super::MouseAction::Selection(outcome) =
        handle_mouse(&mut app, area, event(MouseEventKind::Up(MouseButton::Left)))
    else {
        panic!("pointer release must finish screen selection")
    };
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
    assert_scroll_only_mouse(&mut app);
    assert_eq!(app.input(), "/q");
}

#[test]
fn streaming_commit_deadlines_are_serviced_during_continuous_input_and_completion() {
    use super::RedrawPriority;
    use super::RedrawScheduler;
    use super::advance_stream;
    use super::next_wait;
    use std::time::Duration;
    use zeta_app_server_protocol::protocol::transcript::ThreadTranscriptChange;
    use zeta_app_server_protocol::protocol::transcript::ThreadTranscriptEntry;
    use zeta_app_server_protocol::protocol::transcript::ThreadTranscriptUpdateEnvelope;
    use zeta_protocol::ItemId;
    use zeta_protocol::ThreadItem;
    use zeta_protocol::TurnId;

    let mut app = App::new();
    let turn_id = TurnId::new("stream").unwrap();
    app.set_active_turn(turn_id.clone());
    app.update(ThreadEvent::TranscriptUpdateReceived(Box::new(
        ThreadTranscriptUpdateEnvelope {
            session_id: SessionId::new("session").unwrap(),
            thread_id: ThreadId::new("thread").unwrap(),
            durable_sequence: 1,
            revision: 1,
            stream_cursor: None,
            changes: vec![ThreadTranscriptChange::Upsert {
                entry: ThreadTranscriptEntry::Item {
                    entry_id: "stream-item".into(),
                    turn_id: turn_id.clone(),
                    transient: true,
                    item: ThreadItem::AgentMessage {
                        item_id: ItemId::new("item").unwrap(),
                        turn_id,
                        text: "one\ntwo\nthree\nfour".into(),
                    },
                },
            }],
        },
    )));
    let mut redraw = RedrawScheduler::default();
    let start = app.stream_deadline().unwrap();
    assert_eq!(next_wait(&app, &redraw, start), Some(Duration::ZERO));
    advance_stream(&mut app, &mut redraw, start);
    assert_eq!(app.visible_transcript_views()[0].text(), "one\n");
    assert!(redraw.take_due(start));
    assert_eq!(
        next_wait(&app, &redraw, start),
        Some(Duration::from_millis(40))
    );
    for elapsed in 1..=40 {
        let now = start + Duration::from_millis(elapsed);
        app.insert_text("x");
        redraw.request(now, RedrawPriority::Immediate);
        advance_stream(&mut app, &mut redraw, now);
        assert!(redraw.take_due(now));
    }
    assert_eq!(app.visible_transcript_views()[0].text(), "one\ntwo\n");
    assert_eq!(app.latest_agent_response(), Some("one\ntwo\nthree\nfour"));
    app.update(ThreadEvent::TurnCompleted);
    assert!(matches!(app.status(), crate::app::Status::Ready));
    assert!(app.stream_deadline().is_none());
    assert_eq!(
        app.visible_transcript_views()[0].text(),
        "one\ntwo\nthree\nfour"
    );
}
