use super::App;
use super::AppCommand;
use super::AppEvent;
use super::frame::draw;
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
fn agents_manager_simulates_navigation_and_transient_preview() {
    let mut app = active_session_app();

    assert_eq!(app.handle_key(key(KeyCode::Left)), None);
    assert!(app.session_manager_view().is_some());
    assert!(!app.session_manager_focused());
    assert_snapshot!("agents_manager_open_unfocused", render(&app));

    assert_eq!(app.handle_key(key(KeyCode::Up)), None);
    assert!(app.session_manager_focused());
    assert!(app.session_manager_hint().contains("space to preview"));

    assert_eq!(app.handle_key(key(KeyCode::Char(' '))), None);
    assert_eq!(app.overlay().unwrap().title(), "Session preview");
    assert!(app.session_manager_view().is_some());
    assert_snapshot!("agents_manager_transient_session_preview", render(&app));

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
        Some(AppCommand::ResumeSession {
            session_id: "current".into(),
            preferred_thread_id: Some(ThreadId::new("current").unwrap()),
        })
    );
    app.update(AppEvent::ThreadContextChanged {
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

fn active_session_app() -> App {
    let mut app = App::new();
    app.update(AppEvent::ThreadContextChanged {
        session_id: SessionId::new("current").unwrap(),
        thread_id: ThreadId::new("current").unwrap(),
    });
    app.update(AppEvent::SessionCatalogReceived(vec![session()]));
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
