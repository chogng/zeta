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
use zeta_terminal_detection::TerminalRgb;
use zeta_terminal_detection::detect_host_terminal;

pub(crate) struct TerminalSession {
    background_color: Option<TerminalRgb>,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    modes: TerminalModeGuard<CrosstermModeOperations>,
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
        self.terminal.draw(render).map(|_| ())
    }

    pub(crate) fn area(&self) -> io::Result<Rect> {
        self.terminal
            .size()
            .map(|size| Rect::new(0, 0, size.width, size.height))
    }

    pub(crate) fn capture_mouse(&mut self) -> io::Result<()> {
        self.modes.capture_mouse()
    }

    pub(crate) fn release_mouse(&mut self) -> io::Result<()> {
        self.modes.release_mouse()
    }

    /// Restores the parent terminal, suspends this process, and reacquires TUI modes on resume.
    pub(crate) fn suspend(&mut self) -> io::Result<()> {
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
    mouse_capture_requested: bool,
    mouse_capture: bool,
}

impl<O: TerminalModeOperations> TerminalModeGuard<O> {
    fn acquire(operations: O) -> io::Result<Self> {
        let mut guard = Self {
            operations,
            raw_mode: false,
            alternate_screen: false,
            bracketed_paste: false,
            mouse_capture_requested: false,
            mouse_capture: false,
        };

        guard.reacquire()?;
        Ok(guard)
    }

    fn reacquire(&mut self) -> io::Result<()> {
        let result = (|| {
            self.operations.enable_raw_mode()?;
            self.raw_mode = true;
            self.operations.enter_alternate_screen()?;
            self.alternate_screen = true;
            self.operations.enable_bracketed_paste()?;
            self.bracketed_paste = true;
            if self.mouse_capture_requested {
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

    fn capture_mouse(&mut self) -> io::Result<()> {
        self.mouse_capture_requested = true;
        if self.alternate_screen && !self.mouse_capture {
            self.operations.enable_mouse_capture()?;
            self.mouse_capture = true;
        }
        Ok(())
    }

    fn release_mouse(&mut self) -> io::Result<()> {
        self.mouse_capture_requested = false;
        if self.mouse_capture {
            self.operations.disable_mouse_capture()?;
            self.mouse_capture = false;
        }
        Ok(())
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
