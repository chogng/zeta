use crate::app::App;
use crate::app::AppCommand;
use crate::sessions::Command as SessionCommand;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::Modifier;

fn unstarted_app() -> App {
    App::for_dir_with_input_catalog_and_startup_context(
        std::path::Path::new("."),
        crate::thread::composer::ChatInputCatalog::default(),
        crate::TuiStartupContext::new("."),
    )
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn render(app: &App, width: u16, height: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| crate::app::frame::draw(frame, app))
        .unwrap();
    terminal.backend().buffer().clone()
}

fn text(buffer: &Buffer) -> String {
    buffer
        .content
        .chunks(usize::from(buffer.area.width))
        .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn home_keeps_actions_above_the_fixed_composer() {
    let mut app = unstarted_app();
    app.open_home();
    assert!(app.fullscreen_home_visible());
    assert!(app.messages().is_empty());
    app.handle_key(key(KeyCode::Tab));
    assert_eq!(app.fullscreen.home.selected, Some(0));
    let area = crate::app::fullscreen::layout(&app, ratatui::layout::Rect::new(0, 0, 100, 30));
    let actions = super::layout(area.session.transcript).actions;
    let buffer = render(&app, 100, 30);
    assert_eq!(buffer[(actions.x - 2, actions.y)].symbol(), ">");
    assert_eq!(buffer[(actions.x, actions.y)].symbol(), "R");
    assert_eq!(
        buffer[(actions.x - 2, actions.y)].fg,
        app.render_context().focus()
    );
    assert!(
        buffer[(actions.x, actions.y)]
            .modifier
            .contains(Modifier::BOLD)
    );
    insta::assert_snapshot!("home_actions", text(&buffer));
    app.handle_key(key(KeyCode::Esc));
    assert_eq!(app.fullscreen.home.selected, None);
    app.insert_text("检查项目结构");
    insta::assert_snapshot!("home_draft", text(&render(&app, 100, 30)));
    assert!(app.messages().is_empty());
}

#[test]
fn home_submission_failure_restores_the_complete_draft() {
    let mut app = unstarted_app();
    app.open_home();
    let pasted = "长粘贴内容".repeat(300);
    app.handle_paste(pasted.clone());
    let draft = app.input().to_owned();
    let Some(AppCommand::Sessions(SessionCommand::CreateAndEnter { submission })) =
        app.handle_key(key(KeyCode::Enter))
    else {
        panic!("home input must create a conversation");
    };
    assert!(submission.input.iter().any(|item| matches!(item, crate::thread::composer::ChatInputItem::Text(text) if text.contains(&pasted))));
    assert!(app.sessions.pending_submission.is_some());
    assert!(!app.accepts_input());
    assert_eq!(app.handle_key(key(KeyCode::Enter)), None);
    app.fail_session_creation("Could not create session".into());
    assert_eq!(app.input(), draft);
    assert!(app.accepts_input());
    assert!(app.fullscreen_home_visible());
    insta::assert_snapshot!("home_submission_failed", text(&render(&app, 80, 24)));
}

#[test]
fn home_menu_scrolls_to_every_action_on_short_terminals() {
    let mut app = unstarted_app();
    app.open_home();
    for _ in 0..5 {
        app.handle_key(key(KeyCode::Tab));
    }
    assert_eq!(app.fullscreen.home.selected, Some(4));
    insta::assert_snapshot!("home_narrow", text(&render(&app, 40, 16)));
    assert_eq!(app.handle_key(key(KeyCode::Enter)), Some(AppCommand::Quit));
}

#[test]
fn returning_home_preserves_the_running_task_and_conversation_draft() {
    let mut app = App::new();
    let turn = zeta_protocol::TurnId::new("running-turn").unwrap();
    app.set_active_turn(turn.clone());
    app.update(crate::thread::Event::TurnActivityChanged(
        crate::thread::TurnActivity::Working,
    ));
    app.insert_text("next message");
    app.open_home();
    assert_eq!(app.active_turn(), Some(&turn));
    assert!(app.fullscreen_home_visible());
    app.handle_key(key(KeyCode::Esc));
    assert!(!app.fullscreen_home_visible());
    assert_eq!(app.active_turn(), Some(&turn));
    assert_eq!(app.input(), "next message");
}

#[test]
fn short_home_keeps_the_input_and_selected_action_visible() {
    let mut app = unstarted_app();
    app.open_home();
    let terminal = ratatui::layout::Rect::new(0, 0, 40, 12);
    for _ in 0..5 {
        app.handle_key_in_area(key(KeyCode::Tab), terminal);
    }
    let areas = crate::app::fullscreen::layout(&app, terminal);
    assert_eq!(areas.input.height, 3);
    let buffer = render(&app, 40, 12);
    assert!(text(&buffer).contains("> Quit"));
    assert!(!text(&buffer).contains("Zeta Code"));
    assert!(!text(&buffer).contains("Quit Code"));
    assert_eq!(buffer[(areas.input.x + 2, areas.input.y)].symbol(), "╭");
    insta::assert_snapshot!("home_short", text(&buffer));
}
