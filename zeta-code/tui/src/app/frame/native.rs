use super::Transcript;
use crate::app::App;
use crate::app::welcome;
use crate::terminal::TerminalSession;
use crate::thread::transcript::CellView;
use ratatui::Frame;
use ratatui::layout::Rect;
use std::collections::BTreeSet;
use std::io;
use zeta_protocol::ThreadId;

/// Records terminal output for the currently displayed conversation, independently of redraws.
#[derive(Default)]
pub(crate) struct Output {
    thread: Option<ThreadId>,
    emitted: BTreeSet<String>,
    first: Option<String>,
    header_written: bool,
}

impl Output {
    pub(crate) fn draw(&mut self, terminal: &mut TerminalSession, app: &App) -> io::Result<()> {
        self.select_thread(app.screen_thread_id());
        let screen = terminal.screen_area()?;
        // Release the completed reply's rows before inserting it, so it stays visible above input.
        terminal.set_inline_height(height(app, screen))?;
        if !self.header_written {
            let header = welcome::history_buffer(
                screen.width,
                u16::MAX,
                app.welcome(),
                app.render_context(),
            );
            terminal.append_history(
                usize::from(header.area.height),
                |target, offset, _links| {
                    for row in 0..target.area.height {
                        for column in 0..target.area.width {
                            target[(column, row)] = header[(column, offset as u16 + row)].clone();
                        }
                    }
                },
            )?;
            self.header_written = true;
        }
        if !browsing(app) {
            for view in self.pending(app) {
                let context = app.render_context();
                let cache = app.transcript_render_cache();
                terminal.append_history(
                    view.height(screen.width, context, cache),
                    |buffer, offset, links| {
                        view.render_rows(buffer, offset, context.with_hyperlinks(links), cache);
                    },
                )?;
                self.record(&view);
            }
        }
        terminal.draw(|frame, links| draw(frame, app, links))
    }

    fn pending<'a>(&self, app: &'a App) -> Vec<CellView<'a>> {
        let prefix = app.history_prefix();
        // Older pages stay in the transcript browser, preserving the order of terminal output.
        let start = self
            .first
            .as_ref()
            .and_then(|first| {
                prefix
                    .iter()
                    .position(|cell| cell.cell_id().as_str() == first)
            })
            .unwrap_or_default();
        prefix[start..]
            .iter()
            .filter(|cell| !self.emitted.contains(cell.cell_id().as_str()))
            .map(|cell| cell.history_view())
            .collect()
    }

    pub(crate) fn finish(&mut self, terminal: &mut TerminalSession, app: &App) -> io::Result<()> {
        self.draw(terminal, app)?;
        let width = terminal.screen_area()?.width;
        // Quitting a history browser must also retain output finalized while it was open.
        for view in self.pending(app).into_iter().chain(tail(app)) {
            let context = app.render_context();
            let cache = app.transcript_render_cache();
            terminal.append_history(
                view.height(width, context, cache),
                |buffer, offset, links| {
                    view.render_rows(buffer, offset, context.with_hyperlinks(links), cache);
                },
            )?;
        }
        Ok(())
    }

    fn record(&mut self, view: &CellView<'_>) {
        let id = view
            .cell_id
            .as_ref()
            .expect("history cells have stable identities");
        self.emitted.insert(id.clone());
        self.first.get_or_insert_with(|| id.clone());
    }

    fn select_thread(&mut self, thread: &ThreadId) {
        if self.thread.as_ref() != Some(thread) {
            *self = Self {
                thread: Some(thread.clone()),
                ..Self::default()
            };
        }
    }
}

fn browsing(app: &App) -> bool {
    app.session_preview().is_some()
        || app.session_manager_view().is_some()
        || app.issue_manager().is_some()
        || app.transcript_scroll().anchor().is_some()
        || app.transcript_selection_active()
}

fn tail(app: &App) -> Vec<CellView<'_>> {
    let prefix = app
        .history_prefix()
        .iter()
        .map(|cell| cell.cell_id().as_str())
        .collect::<BTreeSet<_>>();
    app.visible_transcript_views()
        .into_iter()
        .filter(|view| {
            !view
                .cell_id
                .as_deref()
                .is_some_and(|id| prefix.contains(id))
        })
        .collect()
}

pub(super) fn layout(app: &App, area: Rect) -> super::FrameLayout {
    let minimum = if browsing(app) {
        crate::app::layout::MIN_TRANSCRIPT_ROWS
    } else {
        0
    };
    super::fullscreen::layout(app, area, minimum)
}

fn height(app: &App, screen: Rect) -> u16 {
    if browsing(app) || app.overlay().is_some() || super::completion_visible(app) {
        return screen.height;
    }
    let areas = super::layout(app, screen);
    let controls = screen
        .height
        .saturating_sub(areas.session.transcript.height);
    let rows = tail(app).iter().fold(0usize, |rows, cell| {
        rows.saturating_add(cell.height(
            screen.width,
            app.render_context(),
            app.transcript_render_cache(),
        ))
    });
    controls
        .saturating_add(rows.min(u16::MAX as usize) as u16)
        .min(screen.height)
        .max(1)
}

fn draw(
    frame: &mut Frame<'_>,
    app: &App,
    links: &std::cell::RefCell<crate::terminal::hyperlinks::FrameLinks>,
) {
    let transcript = if browsing(app) {
        Transcript::Full
    } else {
        Transcript::Tail(tail(app))
    };
    super::draw_content(frame, app, links, transcript);
}

#[cfg(test)]
#[path = "native_tests.rs"]
mod tests;
