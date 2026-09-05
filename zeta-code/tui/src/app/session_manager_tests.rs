use super::App;
use super::AppCommand;
use super::frame::draw;
use crate::sessions::Command as SessionCommand;
use crate::sessions::Event as SessionEvent;
use crate::thread::Event as ThreadEvent;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use insta::assert_snapshot;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use zeta_protocol::Session;
use zeta_protocol::SessionId;
use zeta_protocol::SessionManagerInfo;
use zeta_protocol::SessionManagerStatus;
use zeta_protocol::SessionStatus;
use zeta_protocol::SessionThread;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadStatus;

const WIDTH: u16 = 100;
const HEIGHT: u16 = 32;

#[test]
fn agents_manager_simulates_navigation_and_transient_details() {
    let mut app = active_session_app();

    assert_eq!(app.handle_key(key(KeyCode::Left)), None);
    assert!(app.session_manager_view().is_some());
    assert!(!app.session_manager_focused());
    assert_snapshot!("agents_manager_open_unfocused", render(&app));

    assert_eq!(app.handle_key(key(KeyCode::Up)), None);
    assert!(app.session_manager_focused());
    assert_eq!(
        app.session_manager_hint(),
        "Enter to open · Space to preview · Ctrl+X to archive · i to details"
    );

    assert_eq!(app.handle_key(key(KeyCode::Char('i'))), None);
    assert_eq!(app.overlay().unwrap().title(), "Session details");
    assert!(app.session_manager_view().is_some());
    assert_snapshot!("agents_manager_transient_session_details", render(&app));

    assert_eq!(app.handle_key(key(KeyCode::Esc)), None);
    assert!(app.overlay().is_none());
    assert!(app.session_manager_focused());
    assert_snapshot!("agents_manager_after_preview_closed", render(&app));

    assert_eq!(app.handle_key(key(KeyCode::Esc)), None);
    assert!(!app.session_manager_focused());
    assert_eq!(app.handle_key(key(KeyCode::Right)), None);
    assert!(app.session_manager_view().is_none());
    assert_eq!(app.screen_navigation_tip(), Some("← for agents"));
}

#[test]
fn resuming_selected_session_restores_manager_navigation() {
    let mut app = active_session_app();
    assert_eq!(app.handle_key(key(KeyCode::Left)), None);
    assert_eq!(app.handle_key(key(KeyCode::Up)), None);

    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Some(AppCommand::Sessions(SessionCommand::Resume {
            session_id: "current".into(),
            preferred_thread_id: Some(ThreadId::new("current").unwrap()),
        }))
    );
    app.update(ThreadEvent::ContextChanged {
        session_id: SessionId::new("current").unwrap(),
        thread_id: ThreadId::new("current").unwrap(),
    });

    assert!(!app.session_manager_focused());
    assert_eq!(app.screen_navigation_tip(), Some("← for agents"));
    assert_snapshot!(
        "agents_session_after_resume_restores_manager_tip",
        render(&app)
    );

    assert_eq!(app.handle_key(key(KeyCode::Left)), None);
    assert!(app.session_manager_view().is_some());
}

#[test]
fn agents_command_opens_the_manager() {
    let mut app = active_session_app();
    app.insert_text("/agents");

    assert!(app.completion().is_some());
    assert_snapshot!("agents_command_completion", render(&app));

    assert_eq!(app.handle_key(key(KeyCode::Enter)), None);
    assert!(app.session_manager_view().is_some());
    assert_snapshot!("agents_command_opened_manager", render(&app));

    assert_eq!(app.handle_key(key(KeyCode::Esc)), None);
    assert!(app.session_manager_view().is_none());
    assert_eq!(app.screen_navigation_tip(), Some("← for agents"));
}

#[test]
fn session_manager_preview_reads_conversation_and_restores_focus_without_editing() {
    let mut app = active_session_app();
    app.handle_key(key(KeyCode::Left));
    app.handle_key(key(KeyCode::Up));
    let draft = app.input().to_owned();
    let Some(AppCommand::Sessions(SessionCommand::Preview { generation, params })) =
        app.handle_key(key(KeyCode::Char(' ')))
    else {
        panic!("preview read expected")
    };
    assert_eq!(params.session_id.as_str(), "current");
    assert_eq!(params.thread_id.as_str(), "current");
    assert!(!app.accepts_input());
    assert_snapshot!("session_manager_preview_loading", render(&app));
    app.finish_session_preview(generation, Ok(preview_result(0..35, false)));
    assert_snapshot!("session_manager_preview_conversation", render(&app));
    for code in [
        KeyCode::Char('x'),
        KeyCode::Char('/'),
        KeyCode::Enter,
        KeyCode::Tab,
    ] {
        assert_eq!(app.handle_key(key(code)), None);
    }
    app.handle_paste("must not enter the draft".into());
    assert_eq!(app.input(), draft);
    assert_eq!(
        app.handle_key_in_area(
            key(KeyCode::PageUp),
            ratatui::layout::Rect::new(0, 0, WIDTH, HEIGHT)
        ),
        None
    );
    assert!(app.session_preview().unwrap().scroll.anchor().is_some());
    assert_snapshot!("session_manager_preview_scrolled", render(&app));
    app.handle_key(key(KeyCode::Esc));
    assert!(app.session_preview().is_none());
    assert!(app.session_manager_focused());
    assert_eq!(app.input(), draft);
    app.finish_session_preview(generation, Ok(preview_result(0..1, false)));
    assert!(app.session_preview().is_none());
    let Some(AppCommand::Sessions(SessionCommand::Preview {
        generation: next, ..
    })) = app.handle_key(key(KeyCode::Char(' ')))
    else {
        panic!("new preview expected")
    };
    assert_ne!(next, generation);
    app.finish_session_preview(generation, Err("stale error".into()));
    assert_eq!(
        app.session_preview().unwrap().notice(),
        Some("Loading conversation…")
    );
    assert!(app.transcript_views().is_empty());
}

#[test]
fn session_manager_pointer_preview_preserves_a_nonempty_draft_and_input_focus() {
    let mut app = active_session_app();
    app.handle_key(key(KeyCode::Left));
    app.insert_text("keep this draft");
    assert!(matches!(
        app.activate_session_manager_pointer_target(
            crate::sessions::SessionManagerPointerTarget::Session(
                SessionId::new("current").unwrap()
            )
        ),
        Some(AppCommand::Sessions(SessionCommand::Preview { .. }))
    ));
    app.handle_key(key(KeyCode::Char('x')));
    app.handle_key(key(KeyCode::Enter));
    app.handle_paste("ignored paste".into());
    app.handle_key(key(KeyCode::Esc));
    assert_eq!(app.input(), "keep this draft");
    assert!(!app.session_manager_focused());
    assert!(app.session_manager_view().is_some());
}

#[test]
fn session_manager_preview_loads_older_history_without_switching_the_active_thread() {
    let mut app = active_session_app();
    app.handle_key(key(KeyCode::Left));
    app.handle_key(key(KeyCode::Up));
    let Some(AppCommand::Sessions(SessionCommand::Preview { generation, .. })) =
        app.handle_key(key(KeyCode::Char(' ')))
    else {
        panic!("preview expected")
    };
    app.finish_session_preview(generation, Ok(preview_result(10..15, true)));
    let Some(AppCommand::Sessions(SessionCommand::Preview { params, .. })) =
        app.handle_key(key(KeyCode::Home))
    else {
        panic!("older history expected")
    };
    assert!(
        matches!(params.history, Some(zeta_app_server_protocol::protocol::session::ThreadSnapshotHistory::Before { turn_id, .. }) if turn_id.as_str() == "turn-10")
    );
    assert_eq!(app.handle_key(key(KeyCode::Home)), None);
    app.finish_session_preview(generation, Ok(preview_result(0..10, false)));
    assert_eq!(app.session_preview().unwrap().messages().len(), 15);
    assert!(app.transcript_views().is_empty());
}

#[test]
fn session_manager_archived_group_restores_deletes_and_previews() {
    let mut app = active_session_app();
    let mut archived = session();
    archived.session_id = SessionId::new("archived").unwrap();
    archived.title = "Archived chat".into();
    archived.status = SessionStatus::Archived;
    archived.threads[0].thread_id = ThreadId::new("archived").unwrap();
    archived.threads[0].status = ThreadStatus::Archived;
    app.update(SessionEvent::CatalogReceived(vec![
        session(),
        archived.clone(),
    ]));
    app.handle_key(key(KeyCode::Left));
    assert!(!render(&app).contains("Archived chat"));
    app.handle_key(key(KeyCode::Up));
    app.handle_key(key(KeyCode::Down));
    assert!(app.session_manager_hint().contains("Enter to expand"));
    app.handle_key(key(KeyCode::Enter));
    app.handle_key(key(KeyCode::Down));
    assert_snapshot!("session_manager_archived_expanded", render(&app));
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Some(
            SessionCommand::Restore {
                session_id: archived.session_id.clone()
            }
            .into()
        )
    );
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL)),
        Some(
            SessionCommand::Delete {
                session_id: archived.session_id.clone()
            }
            .into()
        )
    );
    assert!(
        matches!(app.handle_key(key(KeyCode::Char(' '))), Some(AppCommand::Sessions(SessionCommand::Preview { params, .. })) if params.session_id == archived.session_id)
    );
    app.handle_key(key(KeyCode::Esc));
    archived.status = SessionStatus::Active;
    archived.threads[0].status = ThreadStatus::Active;
    app.update(SessionEvent::CatalogReceived(vec![session(), archived]));
    assert!(app.session_manager_hint().contains("Ctrl+X to archive"));
    assert!(render(&app).contains("Archived (0)"));
}

fn preview_result(
    range: std::ops::Range<usize>,
    has_older_turns: bool,
) -> zeta_app_server_protocol::protocol::session::SessionThreadReadResult {
    use zeta_app_server_protocol::protocol::session::SessionThreadReadResult;
    use zeta_app_server_protocol::protocol::session::ThreadHistoryBoundary;
    use zeta_app_server_protocol::protocol::transcript::ThreadTranscriptEntry;
    use zeta_app_server_protocol::protocol::transcript::ThreadTranscriptSnapshot;
    let thread = zeta_protocol::Thread {
        session_id: SessionId::new("current").unwrap(),
        thread_id: ThreadId::new("current").unwrap(),
        title: "Snapshot session".into(),
        status: ThreadStatus::Active,
        sequence: 42,
        parent_thread_id: None,
        forked_from_id: None,
        usage: Default::default(),
        reference_cost: Default::default(),
        goal: None,
        turns: vec![],
    };
    let boundary = ThreadHistoryBoundary {
        has_older_turns,
        oldest_turn_id: Some(zeta_protocol::TurnId::new(format!("turn-{}", range.start)).unwrap()),
    };
    let entries = range
        .map(|index| {
            let turn_id = zeta_protocol::TurnId::new(format!("turn-{index}")).unwrap();
            ThreadTranscriptEntry::Item {
                entry_id: format!("message-{index}"),
                turn_id: turn_id.clone(),
                transient: false,
                item: zeta_protocol::ThreadItem::AgentMessage {
                    item_id: zeta_protocol::ItemId::new(format!("item-{index}")).unwrap(),
                    turn_id,
                    text: format!(
                        "Conversation message {index:02}: content stays readable in preview."
                    ),
                },
            }
        })
        .collect();
    let transcript = ThreadTranscriptSnapshot {
        session_id: thread.session_id.clone(),
        thread_id: thread.thread_id.clone(),
        durable_sequence: 42,
        revision: 1,
        entries,
    };
    SessionThreadReadResult {
        thread,
        transcript,
        history: Some(boundary),
    }
}

fn active_session_app() -> App {
    let mut app = App::new();
    app.update(ThreadEvent::ContextChanged {
        session_id: SessionId::new("current").unwrap(),
        thread_id: ThreadId::new("current").unwrap(),
    });
    app.update(SessionEvent::CatalogReceived(vec![session()]));
    app
}

fn session() -> Session {
    Session {
        session_id: SessionId::new("current").unwrap(),
        title: "Snapshot session".into(),
        status: SessionStatus::Active,
        manager: SessionManagerInfo {
            status: SessionManagerStatus::Idle,
            status_changed_at_unix_ms: 0,
            activity: None,
            summary: None,
        },
        threads: vec![SessionThread {
            thread_id: ThreadId::new("current").unwrap(),
            title: "main".into(),
            created_at_unix_ms: 0,
            completed_turn_duration_ms: 0,
            active_turn_started_at_unix_ms: None,
            usage: Default::default(),
            parent_thread_id: None,
            forked_from_id: None,
            status: ThreadStatus::Active,
        }],
    }
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn render(app: &App) -> String {
    let backend = TestBackend::new(WIDTH, HEIGHT);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| draw(frame, app)).unwrap();
    let buffer = terminal.backend().buffer();
    (0..HEIGHT)
        .map(|row| {
            (0..WIDTH)
                .map(|column| buffer[(column, row)].symbol())
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
