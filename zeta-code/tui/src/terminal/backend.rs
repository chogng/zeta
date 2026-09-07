use ratatui::backend::Backend;
use ratatui::backend::ClearType;
use ratatui::backend::WindowSize;
use ratatui::buffer::Cell;
use ratatui::layout::Position;
use ratatui::layout::Size;
use std::io;
use std::ops::Range;

/// Keeps interactive redraws out of the main screen's scrollback.
pub(super) struct MainScreenBackend<B> {
    pub(super) inner: B,
}

impl<B: Backend> Backend for MainScreenBackend<B> {
    fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        self.inner.draw(content)
    }

    fn append_lines(&mut self, count: u16) -> io::Result<()> {
        self.inner.append_lines(count)
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        self.inner.hide_cursor()
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        self.inner.show_cursor()
    }

    fn get_cursor_position(&mut self) -> io::Result<Position> {
        self.inner.get_cursor_position()
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> io::Result<()> {
        self.inner.set_cursor_position(position)
    }

    fn clear(&mut self) -> io::Result<()> {
        // VS Code archives the visible screen on ED 2. Home + ED 0 erases
        // the same cells in place, preserving only explicitly committed history.
        self.inner.set_cursor_position(Position::ORIGIN)?;
        self.inner.clear_region(ClearType::AfterCursor)
    }

    fn clear_region(&mut self, region: ClearType) -> io::Result<()> {
        match region {
            ClearType::All => self.clear(),
            _ => self.inner.clear_region(region),
        }
    }

    fn size(&self) -> io::Result<Size> {
        self.inner.size()
    }

    fn window_size(&mut self) -> io::Result<WindowSize> {
        self.inner.window_size()
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }

    fn scroll_region_up(&mut self, region: Range<u16>, amount: u16) -> io::Result<()> {
        self.inner.scroll_region_up(region, amount)
    }

    fn scroll_region_down(&mut self, region: Range<u16>, amount: u16) -> io::Result<()> {
        self.inner.scroll_region_down(region, amount)
    }
}
