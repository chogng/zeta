use crossterm::ExecutableCommand;
use crossterm::event::DisableBracketedPaste;
use crossterm::event::DisableMouseCapture;
use crossterm::event::EnableBracketedPaste;
use crossterm::event::EnableMouseCapture;
use crossterm::terminal::EnterAlternateScreen;
use crossterm::terminal::LeaveAlternateScreen;
use crossterm::terminal::disable_raw_mode;
use crossterm::terminal::enable_raw_mode;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use std::io;
use std::io::Stdout;

pub(crate) struct TerminalSession {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    modes: TerminalModeGuard<CrosstermModeOperations>,
}

impl TerminalSession {
    pub(crate) fn open() -> io::Result<Self> {
        let modes = TerminalModeGuard::acquire(CrosstermModeOperations)?;
        let terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
        let mut session = Self { terminal, modes };
        session.terminal.clear()?;
        Ok(session)
    }

    pub(crate) fn draw<F>(&mut self, render: F) -> io::Result<()>
    where
        F: FnOnce(&mut ratatui::Frame<'_>),
    {
        self.terminal.draw(render).map(|_| ())
    }

    pub(crate) fn area(&self) -> io::Result<Rect> {
        self.terminal
            .size()
            .map(|size| Rect::new(0, 0, size.width, size.height))
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        self.modes.restore();
        let _ = self.terminal.show_cursor();
    }
}

trait TerminalModeOperations {
    fn enable_raw_mode(&mut self) -> io::Result<()>;
    fn enter_alternate_screen(&mut self) -> io::Result<()>;
    fn enable_bracketed_paste(&mut self) -> io::Result<()>;
    fn enable_mouse_capture(&mut self) -> io::Result<()>;
    fn disable_mouse_capture(&mut self) -> io::Result<()>;
    fn disable_bracketed_paste(&mut self) -> io::Result<()>;
    fn leave_alternate_screen(&mut self) -> io::Result<()>;
    fn disable_raw_mode(&mut self) -> io::Result<()>;
}

struct TerminalModeGuard<O: TerminalModeOperations> {
    operations: O,
    raw_mode: bool,
    alternate_screen: bool,
    bracketed_paste: bool,
    mouse_capture: bool,
}

impl<O: TerminalModeOperations> TerminalModeGuard<O> {
    fn acquire(operations: O) -> io::Result<Self> {
        let mut guard = Self {
            operations,
            raw_mode: false,
            alternate_screen: false,
            bracketed_paste: false,
            mouse_capture: false,
        };

        guard.operations.enable_raw_mode()?;
        guard.raw_mode = true;
        guard.operations.enter_alternate_screen()?;
        guard.alternate_screen = true;
        guard.operations.enable_bracketed_paste()?;
        guard.bracketed_paste = true;
        guard.operations.enable_mouse_capture()?;
        guard.mouse_capture = true;
        Ok(guard)
    }

    fn restore(&mut self) {
        if self.mouse_capture {
            let _ = self.operations.disable_mouse_capture();
            self.mouse_capture = false;
        }
        if self.bracketed_paste {
            let _ = self.operations.disable_bracketed_paste();
            self.bracketed_paste = false;
        }
        if self.alternate_screen {
            let _ = self.operations.leave_alternate_screen();
            self.alternate_screen = false;
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

    fn enter_alternate_screen(&mut self) -> io::Result<()> {
        io::stdout().execute(EnterAlternateScreen).map(|_| ())
    }

    fn enable_bracketed_paste(&mut self) -> io::Result<()> {
        io::stdout().execute(EnableBracketedPaste).map(|_| ())
    }

    fn enable_mouse_capture(&mut self) -> io::Result<()> {
        io::stdout().execute(EnableMouseCapture).map(|_| ())
    }

    fn disable_mouse_capture(&mut self) -> io::Result<()> {
        io::stdout().execute(DisableMouseCapture).map(|_| ())
    }

    fn disable_bracketed_paste(&mut self) -> io::Result<()> {
        io::stdout().execute(DisableBracketedPaste).map(|_| ())
    }

    fn leave_alternate_screen(&mut self) -> io::Result<()> {
        io::stdout().execute(LeaveAlternateScreen).map(|_| ())
    }

    fn disable_raw_mode(&mut self) -> io::Result<()> {
        disable_raw_mode()
    }
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
