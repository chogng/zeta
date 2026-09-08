use crate::terminal::backend::MainScreenBackend;
use crate::thread::transcript::CellView;
use crate::thread::transcript::MessageRole;
use ratatui::Terminal;
use ratatui::backend::Backend;
use ratatui::backend::ClearType;
use ratatui::backend::CrosstermBackend;
use ratatui::backend::TestBackend;
use ratatui::backend::WindowSize;
use ratatui::buffer::Cell;
use ratatui::layout::Position;
use ratatui::layout::Size;
use std::io;
use std::path::PathBuf;

// Delegate all emitted bytes to the production backend. Only geometry queries
// are supplied by the test so Terminal uses the real Fullscreen clear path.
struct RecordingBackend<'a> {
    output: CrosstermBackend<&'a mut Vec<u8>>,
    size: Size,
}

impl Backend for RecordingBackend<'_> {
    fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        self.output.draw(content)
    }
    fn append_lines(&mut self, rows: u16) -> io::Result<()> {
        self.output.append_lines(rows)
    }
    fn hide_cursor(&mut self) -> io::Result<()> {
        self.output.hide_cursor()
    }
    fn show_cursor(&mut self) -> io::Result<()> {
        self.output.show_cursor()
    }
    fn get_cursor_position(&mut self) -> io::Result<Position> {
        Ok(Position::ORIGIN)
    }
    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> io::Result<()> {
        self.output.set_cursor_position(position)
    }
    fn clear(&mut self) -> io::Result<()> {
        self.output.clear()
    }
    fn clear_region(&mut self, region: ClearType) -> io::Result<()> {
        self.output.clear_region(region)
    }
    fn size(&self) -> io::Result<Size> {
        Ok(self.size)
    }
    fn window_size(&mut self) -> io::Result<WindowSize> {
        Ok(WindowSize {
            columns_rows: self.size,
            pixels: Size::new(0, 0),
        })
    }
    fn flush(&mut self) -> io::Result<()> {
        Backend::flush(&mut self.output)
    }
    fn scroll_region_up(&mut self, region: std::ops::Range<u16>, amount: u16) -> io::Result<()> {
        self.output.scroll_region_up(region, amount)
    }
    fn scroll_region_down(&mut self, region: std::ops::Range<u16>, amount: u16) -> io::Result<()> {
        self.output.scroll_region_down(region, amount)
    }
}

#[test]
fn history_compatibility_corpus_uses_production_cell_insertion_output() {
    let directory = std::env::var_os("ZETA_TUI_HISTORY_FIXTURES").map(PathBuf::from);
    if let Some(directory) = &directory {
        std::fs::create_dir_all(directory).unwrap();
    }
    let mut cases = Vec::new();
    for (width, height, count) in [
        (80, 24, 120),
        (40, 5, 120),
        (12, 3, 120),
        (120, 40, 120),
        (80, 24, 1),
        (12, 1, 120),
    ] {
        let markers = (0..count)
            .map(|i| format!("ZETA{i:04}中🚀END"))
            .collect::<Vec<_>>();
        let message = CellView::plain(MessageRole::Agent, markers.join("\n"));
        let (cell, rows) = crate::thread::transcript::prepare_history(
            &message,
            width,
            crate::render::test_context(),
        );
        let mut model = Terminal::new(TestBackend::new(width, height)).unwrap();
        model
            .draw(|frame| draw_transient_frame(frame.buffer_mut()))
            .unwrap();
        super::append_history(&mut model, rows, &mut |buffer, area, offset| {
            cell.render(buffer, area, offset)
        })
        .unwrap();
        assert_eq!(
            usize::from(model.backend().scrollback().area.height),
            rows,
            "history must contain exactly the content rows, without empty screens"
        );
        let mut output = Vec::new();
        output.extend_from_slice(b"ZETA-SHELL-SENTINEL\r\n");
        super::begin_screen(&mut output, height).unwrap();
        {
            let backend = RecordingBackend {
                output: CrosstermBackend::new(&mut output),
                size: Size::new(width, height),
            };
            let mut terminal = Terminal::new(MainScreenBackend { inner: backend }).unwrap();
            terminal
                .draw(|frame| draw_transient_frame(frame.buffer_mut()))
                .unwrap();
            super::append_history(&mut terminal, rows, &mut |buffer, area, offset| {
                cell.render(buffer, area, offset)
            })
            .unwrap();
            terminal
                .draw(|frame| {
                    frame
                        .buffer_mut()
                        .set_string(0, 0, "PANEL", ratatui::style::Style::default());
                })
                .unwrap();
        }
        let protocol = String::from_utf8(output.clone()).unwrap();
        assert!(!protocol.contains("\x1b[2J"), "must not clear the screen");
        assert!(
            !protocol.contains("\x1b[3J"),
            "must preserve terminal history"
        );
        assert!(!protocol.contains("?1049"), "must keep the main screen");
        let file = format!("{width}x{height}-{count}.ansi");
        if let Some(directory) = &directory {
            std::fs::write(directory.join(&file), output).unwrap();
        }
        cases.push(serde_json::json!({ "file": file, "width": width, "height": height, "markers": markers }));
    }
    if let Some(directory) = &directory {
        std::fs::write(
            directory.join("manifest.json"),
            serde_json::to_vec_pretty(&cases).unwrap(),
        )
        .unwrap();
    }
}

fn draw_transient_frame(buffer: &mut ratatui::buffer::Buffer) {
    let style = ratatui::style::Style::default();
    buffer.set_string(0, 0, "ZETA-TRANSIENT-WELCOME", style);
    if buffer.area.height > 1 {
        buffer.set_string(0, 1, "ZETA-TRANSIENT-COMPLETION", style);
    }
    if buffer.area.height > 2 {
        buffer.set_string(0, 2, "ZETA-TRANSIENT-INPUT", style);
    }
}

#[test]
fn history_compatibility_status_repaint_never_pushes_the_panel_into_history() {
    let panel = crate::status::status_panel(crate::status::StatusViewData {
        model: "not configured",
        full_context_window: None,
        available_context_window: None,
        remaining_context_window: crate::status::RemainingContextWindow::Unknown,
        usage: &zeta_protocol::ModelUsageSummary::default(),
        reference_cost: &zeta_protocol::ModelReferenceCostSummary::default(),
        session_id: "ZETA-STATUS-SESSION",
        thread_id: "thread-1",
    });
    let mut output = b"ZETA-SHELL-SENTINEL\r\n".to_vec();
    super::begin_screen(&mut output, 40).unwrap();
    let backend = RecordingBackend {
        output: CrosstermBackend::new(&mut output),
        size: Size::new(80, 40),
    };
    let mut terminal = Terminal::new(MainScreenBackend { inner: backend }).unwrap();
    terminal.clear().unwrap();
    let mut steps = Vec::new();
    for stage in ["opened", "repainted", "resized", "cleared", "closed"] {
        match stage {
            "resized" => terminal.backend_mut().inner.size = Size::new(100, 40),
            "cleared" => terminal.clear().unwrap(),
            _ => {}
        }
        terminal
            .draw(|frame| {
                if stage != "closed" {
                    let area = frame.area();
                    let tabs = ratatui::layout::Rect::new(0, 0, area.width, 1);
                    let body = ratatui::layout::Rect::new(0, 1, area.width, area.height - 1);
                    panel.draw_tabs(frame, tabs, None, None, crate::render::test_context());
                    panel.draw_body(frame, body, crate::render::test_context());
                }
            })
            .unwrap();
        let size = terminal.size().unwrap();
        let bytes = std::mem::take(*terminal.backend_mut().inner.output.writer_mut());
        steps.push(serde_json::json!({
            "stage": stage, "width": size.width, "height": size.height,
            "output": String::from_utf8(bytes).unwrap(),
        }));
    }
    if let Some(directory) = std::env::var_os("ZETA_TUI_HISTORY_FIXTURES") {
        let directory = PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            directory.join("status-repaint.json"),
            serde_json::to_vec_pretty(&steps).unwrap(),
        )
        .unwrap();
    }
    for step in steps {
        assert!(
            !step["output"].as_str().unwrap().contains("\x1b[2J"),
            "{}: clearing must not archive the interactive panel",
            step["stage"]
        );
    }
}
