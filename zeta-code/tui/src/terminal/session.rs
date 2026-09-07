use crate::terminal::mouse::MouseMode;
use crate::terminal::screen_selection::ScreenSelectionRange;
use crate::terminal::screen_selection::line_range_at;
use crate::terminal::screen_selection::text_in_range;
use crate::terminal::screen_selection::token_range_at;
use crossterm::ExecutableCommand;
use crossterm::QueueableCommand;
use crossterm::cursor::MoveTo;
use crossterm::event::DisableBracketedPaste;
use crossterm::event::DisableFocusChange;
use crossterm::event::DisableMouseCapture;
use crossterm::event::EnableBracketedPaste;
use crossterm::event::EnableFocusChange;
use crossterm::event::EnableMouseCapture;
use crossterm::terminal::disable_raw_mode;
use crossterm::terminal::enable_raw_mode;
use ratatui::Terminal;
use ratatui::backend::Backend;
use ratatui::backend::CrosstermBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Position;
use ratatui::layout::Rect;
use std::io;
use std::io::Stdout;
use std::io::Write;
use zeta_terminal_detection::TerminalRgb;
use zeta_terminal_detection::detect_host_terminal;

pub(crate) struct TerminalSession {
    background_color: Option<TerminalRgb>,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    modes: TerminalModeGuard<CrosstermModeOperations>,
    rendered_frame: Option<Buffer>,
    cursor_color: CursorColor,
}

impl TerminalSession {
    pub(crate) fn open() -> io::Result<Self> {
        let host_terminal = detect_host_terminal();
        let modes = TerminalModeGuard::acquire(CrosstermModeOperations)?;
        let background_color = super::terminal_probe::query_background(&host_terminal);
        let terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
        let mut session = Self {
            background_color,
            terminal,
            modes,
            rendered_frame: None,
            cursor_color: CursorColor::default(),
        };
        session.terminal.clear()?;
        Ok(session)
    }

    pub(crate) const fn background_color(&self) -> Option<TerminalRgb> {
        self.background_color
    }

    pub(crate) fn draw<F>(&mut self, render: F) -> io::Result<()>
    where
        F: FnOnce(&mut ratatui::Frame<'_>),
    {
        let completed = self.terminal.draw(render)?;
        self.rendered_frame = Some(completed.buffer.clone());
        Ok(())
    }

    pub(crate) fn area(&self) -> io::Result<Rect> {
        self.terminal
            .size()
            .map(|size| Rect::new(0, 0, size.width, size.height))
    }

    pub(crate) fn append_history(
        &mut self,
        rows: usize,
        mut render: impl FnMut(&mut Buffer, usize),
    ) -> io::Result<()> {
        let area = self.area()?;
        append_history(&mut self.terminal, area, rows, &mut render)
    }

    pub(crate) fn set_mouse_mode(&mut self, mode: MouseMode) -> io::Result<()> {
        self.modes.set_mouse_mode(mode)
    }

    pub(crate) fn set_cursor_color(&mut self, color: Option<[u8; 3]>) -> io::Result<()> {
        self.cursor_color.set(&mut io::stdout(), color)
    }

    pub(crate) fn selected_text(&self, range: ScreenSelectionRange) -> Option<String> {
        self.rendered_frame
            .as_ref()
            .and_then(|buffer| text_in_range(buffer, range))
    }

    pub(crate) fn token_range_at(&self, position: Position) -> Option<ScreenSelectionRange> {
        self.rendered_frame
            .as_ref()
            .and_then(|buffer| token_range_at(buffer, position))
    }

    pub(crate) fn line_range_at(&self, position: Position) -> Option<ScreenSelectionRange> {
        self.rendered_frame
            .as_ref()
            .and_then(|buffer| line_range_at(buffer, position))
    }

    /// Restores the parent terminal, suspends this process, and reacquires TUI modes on resume.
    pub(crate) fn suspend(&mut self) -> io::Result<()> {
        self.cursor_color.set(&mut io::stdout(), None)?;
        self.modes.restore();
        let _ = self.terminal.show_cursor();
        let suspend_result = suspend_process();
        let reacquire_result = self.modes.reacquire();
        suspend_result?;
        reacquire_result?;
        self.terminal.clear()
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = self.cursor_color.set(&mut io::stdout(), None);
        self.modes.restore();
        let _ = self.terminal.show_cursor();
    }
}

#[derive(Default)]
struct CursorColor {
    applied: Option<[u8; 3]>,
}

impl CursorColor {
    fn set(&mut self, output: &mut impl Write, color: Option<[u8; 3]>) -> io::Result<()> {
        if self.applied == color {
            return Ok(());
        }
        match color {
            Some([red, green, blue]) => {
                write!(output, "\x1b]12;#{red:02x}{green:02x}{blue:02x}\x1b\\")?
            }
            None => output.write_all(b"\x1b]112\x1b\\")?,
        }
        output.flush()?;
        self.applied = color;
        Ok(())
    }
}

trait TerminalModeOperations {
    fn enable_raw_mode(&mut self) -> io::Result<()>;
    fn begin_screen(&mut self) -> io::Result<()>;
    fn enable_bracketed_paste(&mut self) -> io::Result<()>;
    fn enable_focus_change(&mut self) -> io::Result<()>;
    fn enable_mouse_capture(&mut self) -> io::Result<()>;
    fn disable_mouse_capture(&mut self) -> io::Result<()>;
    fn disable_focus_change(&mut self) -> io::Result<()>;
    fn disable_bracketed_paste(&mut self) -> io::Result<()>;
    fn finish_screen(&mut self) -> io::Result<()>;
    fn disable_raw_mode(&mut self) -> io::Result<()>;
}

struct TerminalModeGuard<O: TerminalModeOperations> {
    operations: O,
    raw_mode: bool,
    screen_active: bool,
    bracketed_paste: bool,
    focus_change: bool,
    mouse_mode: MouseMode,
    mouse_capture: bool,
}

impl<O: TerminalModeOperations> TerminalModeGuard<O> {
    fn acquire(operations: O) -> io::Result<Self> {
        let mut guard = Self {
            operations,
            raw_mode: false,
            screen_active: false,
            bracketed_paste: false,
            focus_change: false,
            mouse_mode: MouseMode::default(),
            mouse_capture: false,
        };

        guard.reacquire()?;
        Ok(guard)
    }

    fn reacquire(&mut self) -> io::Result<()> {
        let result = (|| {
            self.operations.enable_raw_mode()?;
            self.raw_mode = true;
            self.operations.begin_screen()?;
            self.screen_active = true;
            self.operations.enable_bracketed_paste()?;
            self.bracketed_paste = true;
            self.operations.enable_focus_change()?;
            self.focus_change = true;
            if self.mouse_mode == MouseMode::TuiCapture {
                self.operations.enable_mouse_capture()?;
                self.mouse_capture = true;
            }
            Ok(())
        })();
        if result.is_err() {
            self.restore();
        }
        result
    }

    fn set_mouse_mode(&mut self, mode: MouseMode) -> io::Result<()> {
        if self.mouse_mode == mode {
            return Ok(());
        }
        match mode {
            MouseMode::TerminalSelection => {
                if self.mouse_capture {
                    self.operations.disable_mouse_capture()?;
                    self.mouse_capture = false;
                }
            }
            MouseMode::TuiCapture => {
                if self.screen_active && !self.mouse_capture {
                    self.operations.enable_mouse_capture()?;
                    self.mouse_capture = true;
                }
            }
        }
        self.mouse_mode = mode;
        Ok(())
    }

    fn restore(&mut self) {
        if self.mouse_capture {
            let _ = self.operations.disable_mouse_capture();
            self.mouse_capture = false;
        }
        if self.focus_change {
            let _ = self.operations.disable_focus_change();
            self.focus_change = false;
        }
        if self.bracketed_paste {
            let _ = self.operations.disable_bracketed_paste();
            self.bracketed_paste = false;
        }
        if self.screen_active {
            let _ = self.operations.finish_screen();
            self.screen_active = false;
        }
        if self.raw_mode {
            let _ = self.operations.disable_raw_mode();
            self.raw_mode = false;
        }
    }
}

impl<O: TerminalModeOperations> Drop for TerminalModeGuard<O> {
    fn drop(&mut self) {
        self.restore();
    }
}

struct CrosstermModeOperations;

impl TerminalModeOperations for CrosstermModeOperations {
    fn enable_raw_mode(&mut self) -> io::Result<()> {
        enable_raw_mode()
    }

    fn begin_screen(&mut self) -> io::Result<()> {
        let (_, rows) = crossterm::terminal::size()?;
        begin_screen(&mut io::stdout(), rows)
    }

    fn enable_bracketed_paste(&mut self) -> io::Result<()> {
        io::stdout().execute(EnableBracketedPaste).map(|_| ())
    }

    fn enable_focus_change(&mut self) -> io::Result<()> {
        io::stdout().execute(EnableFocusChange).map(|_| ())
    }

    fn enable_mouse_capture(&mut self) -> io::Result<()> {
        io::stdout().execute(EnableMouseCapture).map(|_| ())
    }

    fn disable_mouse_capture(&mut self) -> io::Result<()> {
        io::stdout().execute(DisableMouseCapture).map(|_| ())
    }

    fn disable_focus_change(&mut self) -> io::Result<()> {
        io::stdout().execute(DisableFocusChange).map(|_| ())
    }

    fn disable_bracketed_paste(&mut self) -> io::Result<()> {
        io::stdout().execute(DisableBracketedPaste).map(|_| ())
    }

    fn finish_screen(&mut self) -> io::Result<()> {
        let (_, rows) = crossterm::terminal::size()?;
        finish_screen(&mut io::stdout(), rows)
    }

    fn disable_raw_mode(&mut self) -> io::Result<()> {
        disable_raw_mode()
    }
}

fn append_history<B: Backend>(
    terminal: &mut Terminal<B>,
    area: Rect,
    rows: usize,
    render: &mut impl FnMut(&mut Buffer, usize),
) -> io::Result<()> {
    if area.is_empty() {
        return Err(io::Error::other(
            "terminal has no space for transcript output",
        ));
    }
    terminal.clear()?;
    // Emit each rendered row with its own line feed. Painting an entire page
    // and then scrolling it loses the last painted row through Windows ConPTY.
    // Start at the top so short messages do not archive an empty screen first.
    for offset in 0..rows {
        let chunk = Rect::new(0, 0, area.width, 1);
        let empty = Buffer::empty(chunk);
        let mut buffer = empty.clone();
        render(&mut buffer, offset);
        let backend = terminal.backend_mut();
        let y = offset.min(usize::from(area.height - 1)) as u16;
        backend.draw(
            empty
                .diff(&buffer)
                .into_iter()
                .map(|(x, _, cell)| (x, y, cell)),
        )?;
        backend.set_cursor_position((0, y))?;
        backend.append_lines(1)?;
        backend.flush()?;
    }
    // Commit the rows still on screen before clearing the interactive viewport.
    terminal
        .backend_mut()
        .set_cursor_position((0, area.height - 1))?;
    terminal
        .backend_mut()
        .append_lines(rows.min(usize::from(area.height - 1)) as u16)?;
    terminal.clear()
}

// Keep the main buffer: xterm.js translates wheel input into arrow keys in
// the alternate buffer whenever mouse reporting is disabled. Reserving fresh
// rows also keeps the caller's output in terminal scrollback.
fn begin_screen(output: &mut impl Write, rows: u16) -> io::Result<()> {
    output.queue(MoveTo(0, rows.saturating_sub(1)))?;
    for _ in 0..rows {
        output.write_all(b"\r\n")?;
    }
    output.queue(MoveTo(0, 0))?;
    output.flush()
}

fn finish_screen(output: &mut impl Write, rows: u16) -> io::Result<()> {
    output.queue(MoveTo(0, rows.saturating_sub(1)))?;
    output.write_all(b"\r\n")?;
    output.flush()
}

#[cfg(unix)]
fn suspend_process() -> io::Result<()> {
    rustix::process::kill_process(rustix::process::getpid(), rustix::process::Signal::TSTP)
        .map_err(io::Error::from)
}

#[cfg(not(unix))]
fn suspend_process() -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "process suspension is unsupported on this platform",
    ))
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "history_protocol_tests.rs"]
mod history_protocol_tests;
