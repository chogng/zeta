use super::*;
use crate::config::Event as ConfigEvent;
use crate::config::TerminalSettings;
use crate::terminal::ScreenMode;
use crate::terminal::mouse::MouseMode;
use crate::thread::Event as ThreadEvent;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use zeta_app_server_protocol::protocol::transcript::ThreadTranscriptEntry;
use zeta_app_server_protocol::protocol::transcript::ThreadTranscriptSnapshot;
use zeta_protocol::ItemId;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadItem;
use zeta_protocol::TurnId;

fn app() -> App {
    let mut app = App::new();
    let mut settings = TerminalSettings::default();
    settings.set_screen_mode(ScreenMode::Native);
    app.update(ConfigEvent::SettingsReceived(settings));
    app
}

fn entry(id: &str, text: &str, transient: bool) -> ThreadTranscriptEntry {
    let turn_id = TurnId::new(id).unwrap();
    ThreadTranscriptEntry::Item {
        entry_id: id.into(),
        turn_id: turn_id.clone(),
        item: ThreadItem::AgentMessage {
            item_id: ItemId::new(id).unwrap(),
            turn_id,
            text: text.into(),
        },
        transient,
    }
}

fn snapshot(entries: Vec<ThreadTranscriptEntry>) -> ThreadTranscriptSnapshot {
    ThreadTranscriptSnapshot {
        session_id: SessionId::new("session").unwrap(),
        thread_id: ThreadId::new("thread").unwrap(),
        durable_sequence: 1,
        revision: 1,
        entries,
    }
}

fn render(app: &App, width: u16, rows: u16) -> Buffer {
    let rows = height(app, Rect::new(0, 0, width, rows));
    let mut terminal = Terminal::new(TestBackend::new(width, rows)).unwrap();
    terminal
        .draw(|frame| draw(frame, app, &Default::default()))
        .unwrap();
    terminal.backend().buffer().clone()
}

fn text(buffer: &Buffer) -> String {
    buffer
        .content
        .chunks(usize::from(buffer.area.width))
        .map(|row| {
            let mut line = String::new();
            let mut continuation = 0;
            for cell in row {
                if continuation > 0 {
                    continuation -= 1;
                    continue;
                }
                line.push_str(cell.symbol());
                continuation =
                    unicode_width::UnicodeWidthStr::width(cell.symbol()).saturating_sub(1);
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn native_input_keeps_terminal_selection_and_uses_a_bounded_area() {
    let mut app = app();
    assert_eq!(app.mouse_mode(), MouseMode::TerminalSelection);
    assert!(!app.mouse_mode().enables_pointer_actions());
    app.insert_text("继续检查终端历史");
    let buffer = render(&app, 80, 32);
    assert!(buffer.area.height < 32);
    assert!(text(&buffer).contains("继续检查终端历史"));
    assert!(!text(&buffer).contains("Zeta Code v"));
    insta::assert_snapshot!("native_input", text(&buffer));
    let layout = super::super::layout(&app, buffer.area);
    assert!(layout.input.height > 0);
    assert!(layout.input.bottom() <= buffer.area.bottom());
    let mut settings = TerminalSettings::default();
    settings.set_screen_mode(ScreenMode::Fullscreen);
    app.update(ConfigEvent::SettingsReceived(settings));
    assert_eq!(app.mouse_mode(), MouseMode::TuiCapture);
    assert!(app.input().contains("继续检查终端历史"));
}

#[test]
fn native_transcript_retains_active_turn_until_completion_and_emits_once() {
    let mut app = app();
    let old = entry("old", "Earlier completed answer", false);
    let current = entry("current", "当前回复\n\n- 第一项\n- 第二项", false);
    app.update(ThreadEvent::TranscriptSnapshotReceived(snapshot(vec![
        old.clone(),
        current.clone(),
    ])));
    app.set_active_turn(TurnId::new("current").unwrap());
    let mut output = Output::default();
    output.select_thread(app.screen_thread_id());
    let pending = output.pending(&app);
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].text(), "Earlier completed answer");
    output.record(&pending[0]);
    assert!(output.pending(&app).is_empty());
    assert_eq!(tail(&app).len(), 1);
    insta::assert_snapshot!("native_current_reply", text(&render(&app, 60, 24)));
    app.clear_active_turn();
    let pending = output.pending(&app);
    assert_eq!(pending.len(), 1);
    assert!(pending[0].text().starts_with("当前回复"));
    output.record(&pending[0]);
    app.update(ThreadEvent::TranscriptSnapshotReceived(snapshot(vec![
        old, current,
    ])));
    assert!(
        output.pending(&app).is_empty(),
        "resynchronization must not duplicate history"
    );
    assert!(tail(&app).is_empty());
    app.update(ThreadEvent::TranscriptHistoryPageReceived(snapshot(vec![
        entry("older", "Older page", false),
    ])));
    assert!(
        output.pending(&app).is_empty(),
        "older pages must not be appended out of order"
    );
    assert!(tail(&app).is_empty());
    app.handle_key_in_area(
        KeyEvent::new(KeyCode::Home, KeyModifiers::CONTROL),
        Rect::new(0, 0, 60, 24),
    );
    assert!(browsing(&app));
    assert!(text(&render(&app, 60, 24)).contains("Older page"));
    app.handle_key_in_area(
        KeyEvent::new(KeyCode::End, KeyModifiers::CONTROL),
        Rect::new(0, 0, 60, 24),
    );
    assert!(!browsing(&app));
}

#[test]
fn native_config_opens_and_closes_without_reprinting_history() {
    let mut app = app();
    let choices = crate::config::config_choices(
        &crate::test_support::empty_config_snapshot(),
        &zeta_app_server_protocol::protocol::provider::ProviderListResult { providers: vec![] },
        {
            let mut settings = TerminalSettings::default();
            settings.set_screen_mode(ScreenMode::Native);
            settings
        },
        crate::status::StatusLineSettings::default(),
    );
    app.update(ConfigEvent::EditorOpened(choices));
    assert!(app.command_panel().is_some());
    let buffer = render(&app, 100, 32);
    assert!(text(&buffer).contains("Screen mode"));
    assert!(text(&buffer).contains("native"));
    insta::assert_snapshot!("native_config", text(&buffer));
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(app.command_panel().is_none());
    insta::assert_snapshot!("native_config_closed", text(&render(&app, 100, 32)));
}
