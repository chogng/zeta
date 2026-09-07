use super::TerminalModeGuard;
use super::TerminalModeOperations;
use crate::terminal::mouse::MouseMode;
use std::cell::RefCell;
use std::io;
use std::rc::Rc;

fn fullscreen_terminal(
    width: u16,
    height: u16,
) -> ratatui::Terminal<ratatui::backend::TestBackend> {
    ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, height)).unwrap()
}

const ENABLE_RAW_MODE: &str = "enable raw mode";
const BEGIN_SCREEN: &str = "begin screen";
const ENABLE_BRACKETED_PASTE: &str = "enable bracketed paste";
const ENABLE_FOCUS_CHANGE: &str = "enable focus change";
const ENABLE_MOUSE_CAPTURE: &str = "enable mouse capture";
const DISABLE_MOUSE_CAPTURE: &str = "disable mouse capture";
const DISABLE_FOCUS_CHANGE: &str = "disable focus change";
const DISABLE_BRACKETED_PASTE: &str = "disable bracketed paste";
const FINISH_SCREEN: &str = "finish screen";
const DISABLE_RAW_MODE: &str = "disable raw mode";

#[test]
fn transcript_output_protocol_keeps_the_main_buffer_and_existing_history() {
    use crate::thread::transcript::CellView;
    use crate::thread::transcript::MessageRole;
    use ratatui::Terminal;
    use ratatui::TerminalOptions;
    use ratatui::Viewport;
    use ratatui::backend::CrosstermBackend;
    let mut output = Vec::new();
    let message = CellView::plain(
        MessageRole::Agent,
        (0..120).map(|i| format!("history {i:03}\n")).collect(),
    );
    let (cell, rows) =
        crate::thread::transcript::prepare_history(&message, 40, crate::render::test_context());
    {
        let backend = CrosstermBackend::new(&mut output);
        let mut terminal = Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Inline(5),
            },
        )
        .unwrap();
        super::append_history(&mut terminal, None, rows, &mut |buffer, area, offset| {
            cell.render(buffer, area, offset)
        })
        .unwrap();
    }
    let text = String::from_utf8(output.clone()).unwrap();
    assert!(!text.contains("\x1b[3J"));
    assert!(!text.contains("?1049"));
    if let Some(path) = std::env::var_os("ZETA_TUI_HISTORY_TRACE") {
        std::fs::write(path, output).unwrap();
    }
}

#[test]
fn history_longer_than_the_screen_survives_repainting_and_resize() {
    use ratatui::style::Style;
    let mut terminal = fullscreen_terminal(40, 5);
    super::append_history(&mut terminal, None, 101, &mut |buffer, area, offset| {
        buffer.set_string(
            area.x,
            area.y,
            format!("message {offset} 中文 🚀"),
            Style::default(),
        );
    })
    .unwrap();
    assert_eq!(terminal.backend().scrollback().area.height, 101);
    terminal
        .draw(|frame| {
            frame
                .buffer_mut()
                .set_string(0, 0, "Config panel", Style::default());
        })
        .unwrap();
    terminal.backend_mut().resize(40, 8);
    terminal.autoresize().unwrap();
    let history = terminal.backend().scrollback();
    assert_eq!(history.area.height, 101);
    for row in 0..101_u16 {
        let text = (0..40)
            .map(|column| history[(column, row)].symbol())
            .collect::<String>();
        let prefix = format!("message {row} ");
        assert!(text.starts_with(&prefix), "{row}: {text}");
        assert_eq!(history[(prefix.len() as u16, row)].symbol(), "中");
        assert_eq!(history[(prefix.len() as u16 + 2, row)].symbol(), "文");
    }
}

#[test]
fn inserting_history_never_commits_the_interactive_frame() {
    use ratatui::style::Style;

    let mut terminal = fullscreen_terminal(40, 6);
    terminal
        .draw(|frame| {
            frame
                .buffer_mut()
                .set_string(0, 0, "WELCOME", Style::default());
            frame
                .buffer_mut()
                .set_string(0, 2, "SLASH COMPLETION", Style::default());
            frame
                .buffer_mut()
                .set_string(0, 5, "> /statusline", Style::default());
        })
        .unwrap();

    let rendered = terminal.backend().buffer().clone();
    super::append_history(&mut terminal, Some(&rendered), 1, &mut |buffer, area, _| {
        buffer.set_string(area.x, area.y, "> /statusline", Style::default());
    })
    .unwrap();

    let history = terminal.backend().scrollback();
    let text = (0..history.area.height)
        .map(|row| {
            (0..history.area.width)
                .map(|column| history[(column, row)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        text.matches("> /statusline").count(),
        1,
        "unexpected scrollback: {text:?}"
    );
    assert!(!text.contains("WELCOME"));
    assert!(!text.contains("SLASH COMPLETION"));
}

#[test]
fn complete_styled_answer_is_written_to_scrollback_in_small_chunks() {
    use crate::thread::transcript::CellView;
    use crate::thread::transcript::MessageRole;
    let text = (0..200)
        .map(|row| format!("line {row:03} 中文\n"))
        .collect::<String>();
    let message = CellView::plain(MessageRole::Agent, text);
    let (cell, rows) =
        crate::thread::transcript::prepare_history(&message, 40, crate::render::test_context());
    let mut terminal = fullscreen_terminal(40, 5);
    super::append_history(&mut terminal, None, rows, &mut |buffer, area, offset| {
        cell.render(buffer, area, offset);
    })
    .unwrap();
    let history = terminal.backend().scrollback();
    let lines = (0..history.area.height)
        .map(|row| {
            (0..history.area.width)
                .map(|column| history[(column, row)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>();
    for row in 0..200 {
        assert_eq!(
            lines
                .iter()
                .filter(|line| line.contains(&format!("line {row:03}")))
                .count(),
            1
        );
    }
    assert!(lines.last().unwrap().trim().is_empty());
}

#[test]
fn cursor_color_updates_once_and_resets_before_reapplication() {
    let mut color = super::CursorColor::default();
    let mut output = Vec::new();
    color.set(&mut output, Some([0x66, 0x58, 0xc7])).unwrap();
    color.set(&mut output, Some([0x66, 0x58, 0xc7])).unwrap();
    color.set(&mut output, None).unwrap();
    color.set(&mut output, None).unwrap();
    color.set(&mut output, Some([0x11, 0x22, 0x33])).unwrap();
    assert_eq!(
        output,
        b"\x1b]12;#6658c7\x1b\\\x1b]112\x1b\\\x1b]12;#112233\x1b\\"
    );
}

#[test]
fn acquired_terminal_modes_are_restored_in_reverse_order() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let guard =
        TerminalModeGuard::acquire(FakeOperations::new(calls.clone(), None)).expect("acquire");

    drop(guard);

    assert_eq!(
        calls.borrow().as_slice(),
        [
            ENABLE_RAW_MODE,
            BEGIN_SCREEN,
            ENABLE_BRACKETED_PASTE,
            ENABLE_FOCUS_CHANGE,
            DISABLE_FOCUS_CHANGE,
            DISABLE_BRACKETED_PASTE,
            FINISH_SCREEN,
            DISABLE_RAW_MODE,
        ]
    );
}

#[test]
fn acquisition_failure_restores_only_modes_that_were_acquired() {
    let cases = [
        (ENABLE_RAW_MODE, vec![ENABLE_RAW_MODE]),
        (
            BEGIN_SCREEN,
            vec![ENABLE_RAW_MODE, BEGIN_SCREEN, DISABLE_RAW_MODE],
        ),
        (
            ENABLE_BRACKETED_PASTE,
            vec![
                ENABLE_RAW_MODE,
                BEGIN_SCREEN,
                ENABLE_BRACKETED_PASTE,
                FINISH_SCREEN,
                DISABLE_RAW_MODE,
            ],
        ),
        (
            ENABLE_FOCUS_CHANGE,
            vec![
                ENABLE_RAW_MODE,
                BEGIN_SCREEN,
                ENABLE_BRACKETED_PASTE,
                ENABLE_FOCUS_CHANGE,
                DISABLE_BRACKETED_PASTE,
                FINISH_SCREEN,
                DISABLE_RAW_MODE,
            ],
        ),
    ];

    for (failure, expected_calls) in cases {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let result = TerminalModeGuard::acquire(FakeOperations::new(calls.clone(), Some(failure)));

        assert!(result.is_err(), "{failure} should fail");
        assert_eq!(
            calls.borrow().as_slice(),
            expected_calls,
            "unexpected rollback for {failure}"
        );
    }
}

#[test]
fn mouse_mode_is_applied_idempotently() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut guard =
        TerminalModeGuard::acquire(FakeOperations::new(calls.clone(), None)).expect("acquire");

    guard
        .set_mouse_mode(MouseMode::TuiCapture)
        .expect("enable TUI pointer capture");
    guard
        .set_mouse_mode(MouseMode::TuiCapture)
        .expect("keep TUI pointer capture enabled");
    guard
        .set_mouse_mode(MouseMode::TerminalSelection)
        .expect("restore terminal selection");
    guard
        .set_mouse_mode(MouseMode::TerminalSelection)
        .expect("keep terminal selection restored");
    drop(guard);

    assert_eq!(
        calls.borrow().as_slice(),
        [
            ENABLE_RAW_MODE,
            BEGIN_SCREEN,
            ENABLE_BRACKETED_PASTE,
            ENABLE_FOCUS_CHANGE,
            ENABLE_MOUSE_CAPTURE,
            DISABLE_MOUSE_CAPTURE,
            DISABLE_FOCUS_CHANGE,
            DISABLE_BRACKETED_PASTE,
            FINISH_SCREEN,
            DISABLE_RAW_MODE,
        ]
    );
}

#[test]
fn explicit_restore_is_idempotent() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut guard =
        TerminalModeGuard::acquire(FakeOperations::new(calls.clone(), None)).expect("acquire");

    guard.restore();
    guard.restore();
    drop(guard);

    assert_eq!(
        calls.borrow().as_slice(),
        [
            ENABLE_RAW_MODE,
            BEGIN_SCREEN,
            ENABLE_BRACKETED_PASTE,
            ENABLE_FOCUS_CHANGE,
            DISABLE_FOCUS_CHANGE,
            DISABLE_BRACKETED_PASTE,
            FINISH_SCREEN,
            DISABLE_RAW_MODE,
        ]
    );
}

#[test]
fn suspend_cycle_reacquires_requested_mouse_capture() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut guard =
        TerminalModeGuard::acquire(FakeOperations::new(calls.clone(), None)).expect("acquire");

    guard
        .set_mouse_mode(MouseMode::TuiCapture)
        .expect("enable TUI pointer capture");
    guard.restore();
    guard.reacquire().expect("reacquire");
    drop(guard);

    assert_eq!(
        calls.borrow().as_slice(),
        [
            ENABLE_RAW_MODE,
            BEGIN_SCREEN,
            ENABLE_BRACKETED_PASTE,
            ENABLE_FOCUS_CHANGE,
            ENABLE_MOUSE_CAPTURE,
            DISABLE_MOUSE_CAPTURE,
            DISABLE_FOCUS_CHANGE,
            DISABLE_BRACKETED_PASTE,
            FINISH_SCREEN,
            DISABLE_RAW_MODE,
            ENABLE_RAW_MODE,
            BEGIN_SCREEN,
            ENABLE_BRACKETED_PASTE,
            ENABLE_FOCUS_CHANGE,
            ENABLE_MOUSE_CAPTURE,
            DISABLE_MOUSE_CAPTURE,
            DISABLE_FOCUS_CHANGE,
            DISABLE_BRACKETED_PASTE,
            FINISH_SCREEN,
            DISABLE_RAW_MODE,
        ]
    );
}

struct FakeOperations {
    calls: Rc<RefCell<Vec<&'static str>>>,
    failure: Option<&'static str>,
}

impl FakeOperations {
    fn new(calls: Rc<RefCell<Vec<&'static str>>>, failure: Option<&'static str>) -> Self {
        Self { calls, failure }
    }

    fn call(&self, operation: &'static str) -> io::Result<()> {
        self.calls.borrow_mut().push(operation);
        if self.failure == Some(operation) {
            Err(io::Error::other(format!("{operation} failed")))
        } else {
            Ok(())
        }
    }
}

impl TerminalModeOperations for FakeOperations {
    fn enable_raw_mode(&mut self) -> io::Result<()> {
        self.call(ENABLE_RAW_MODE)
    }

    fn begin_screen(&mut self) -> io::Result<()> {
        self.call(BEGIN_SCREEN)
    }

    fn enable_bracketed_paste(&mut self) -> io::Result<()> {
        self.call(ENABLE_BRACKETED_PASTE)
    }

    fn enable_focus_change(&mut self) -> io::Result<()> {
        self.call(ENABLE_FOCUS_CHANGE)
    }

    fn enable_mouse_capture(&mut self) -> io::Result<()> {
        self.call(ENABLE_MOUSE_CAPTURE)
    }

    fn disable_mouse_capture(&mut self) -> io::Result<()> {
        self.call(DISABLE_MOUSE_CAPTURE)
    }

    fn disable_focus_change(&mut self) -> io::Result<()> {
        self.call(DISABLE_FOCUS_CHANGE)
    }

    fn disable_bracketed_paste(&mut self) -> io::Result<()> {
        self.call(DISABLE_BRACKETED_PASTE)
    }

    fn finish_screen(&mut self) -> io::Result<()> {
        self.call(FINISH_SCREEN)
    }

    fn disable_raw_mode(&mut self) -> io::Result<()> {
        self.call(DISABLE_RAW_MODE)
    }
}

#[test]
fn screen_output_preserves_main_buffer_and_leaves_a_prompt_line() {
    let mut output = Vec::new();
    super::begin_screen(&mut output, 3).unwrap();
    output.extend_from_slice(b"panel");
    super::finish_screen(&mut output, 3).unwrap();
    assert_eq!(output, b"\x1b[3;1H\r\n\r\n\r\n\x1b[1;1Hpanel\x1b[3;1H\r\n");
}
