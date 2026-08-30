use crate::ui::accent;
use crate::ui::muted;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use zeta_protocol::Session;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadStatus;

pub(crate) const DEFAULT_MAX_ROWS: usize = 4;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SubagentPaneRow {
    pub(crate) thread_id: ThreadId,
    pub(crate) label: String,
}

#[derive(Debug, Default)]
pub(crate) struct SubagentPaneState {
    rows: Vec<SubagentPaneRow>,
    selected: Option<ThreadId>,
    viewport_start: usize,
    focused: bool,
}

impl SubagentPaneState {
    pub(crate) fn reconcile(
        &mut self,
        session: Option<&Session>,
        viewed_thread: Option<&ThreadId>,
    ) {
        self.rows = session.map(active_rows).unwrap_or_default();
        let selected_is_valid = self
            .selected
            .as_ref()
            .is_some_and(|selected| self.rows.iter().any(|row| &row.thread_id == selected));
        if !selected_is_valid {
            self.selected = viewed_thread
                .filter(|viewed| self.rows.iter().any(|row| &row.thread_id == *viewed))
                .cloned()
                .or_else(|| self.rows.first().map(|row| row.thread_id.clone()));
        }
        self.keep_selection_visible(DEFAULT_MAX_ROWS);
        if self.rows.is_empty() {
            self.focused = false;
        }
    }

    pub(crate) fn focus(&mut self) -> bool {
        if self.rows.is_empty() {
            return false;
        }
        self.focused = true;
        true
    }

    pub(crate) fn blur(&mut self) {
        self.focused = false;
    }

    pub(crate) fn focused(&self) -> bool {
        self.focused
    }

    pub(crate) fn select_previous(&mut self) -> bool {
        let Some(index) = self.selected_index() else {
            return false;
        };
        if index == 0 {
            return false;
        }
        self.selected = Some(self.rows[index - 1].thread_id.clone());
        self.keep_selection_visible(DEFAULT_MAX_ROWS);
        true
    }

    pub(crate) fn select_next(&mut self) -> bool {
        let Some(index) = self.selected_index() else {
            return false;
        };
        let Some(row) = self.rows.get(index.saturating_add(1)) else {
            return false;
        };
        self.selected = Some(row.thread_id.clone());
        self.keep_selection_visible(DEFAULT_MAX_ROWS);
        true
    }

    pub(crate) fn selected(&self) -> Option<&ThreadId> {
        self.selected.as_ref()
    }

    pub(crate) fn view(&self) -> SubagentPaneView<'_> {
        let end = self
            .viewport_start
            .saturating_add(DEFAULT_MAX_ROWS)
            .min(self.rows.len());
        SubagentPaneView {
            rows: &self.rows[self.viewport_start..end],
            selected: self.selected.as_ref(),
            focused: self.focused,
        }
    }

    pub(crate) fn desired_rows(&self) -> u16 {
        u16::try_from(self.rows.len().min(DEFAULT_MAX_ROWS)).unwrap_or(u16::MAX)
    }

    fn selected_index(&self) -> Option<usize> {
        let selected = self.selected.as_ref()?;
        self.rows.iter().position(|row| &row.thread_id == selected)
    }

    fn keep_selection_visible(&mut self, max_rows: usize) {
        let Some(index) = self.selected_index() else {
            self.viewport_start = 0;
            return;
        };
        if index < self.viewport_start {
            self.viewport_start = index;
        } else if index >= self.viewport_start.saturating_add(max_rows) {
            self.viewport_start = index.saturating_add(1).saturating_sub(max_rows);
        }
    }
}

pub(crate) struct SubagentPaneView<'a> {
    pub(crate) rows: &'a [SubagentPaneRow],
    pub(crate) selected: Option<&'a ThreadId>,
    pub(crate) focused: bool,
}

pub(crate) fn draw_subagent_pane(frame: &mut Frame<'_>, area: Rect, view: SubagentPaneView<'_>) {
    let lines = view
        .rows
        .iter()
        .map(|row| {
            let selected = view.selected == Some(&row.thread_id);
            let marker = if view.focused && selected { ">" } else { " " };
            let style = if selected {
                Style::default().fg(accent())
            } else {
                Style::default().fg(muted())
            };
            Line::from(vec![
                Span::styled(format!("{marker} {}", row.label), style),
                Span::styled(format!("  {}", row.thread_id), Style::default().fg(muted())),
            ])
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines), area);
}

fn active_rows(session: &Session) -> Vec<SubagentPaneRow> {
    let mut rows = Vec::new();
    if let Some(root) = session.threads.iter().find(|thread| {
        thread.thread_id.as_str() == session.session_id.as_str()
            && thread.status == ThreadStatus::Active
    }) {
        rows.push(SubagentPaneRow {
            thread_id: root.thread_id.clone(),
            label: "Main".into(),
        });
    }
    rows.extend(
        session
            .threads
            .iter()
            .filter(|thread| {
                thread.status == ThreadStatus::Active
                    && thread.parent_thread_id.is_some()
                    && thread.forked_from_id.is_none()
            })
            .map(|thread| SubagentPaneRow {
                thread_id: thread.thread_id.clone(),
                label: thread.thread_id.to_string(),
            }),
    );
    rows
}

#[cfg(test)]
#[path = "subagent_pane_tests.rs"]
mod tests;
