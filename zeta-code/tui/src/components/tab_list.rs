use crate::ui::muted;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use unicode_width::UnicodeWidthStr;

const TAB_GAP: usize = 2;

/// Supplies the label rendered by a tab list while the owning component keeps its payload.
pub(crate) trait TabListItem {
    fn tab_label(&self) -> &str;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TabListInputOutcome {
    ActiveChanged,
    Consumed,
    Unhandled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TabListState<T> {
    tabs: Vec<T>,
    active: usize,
}

impl<T> TabListState<T> {
    pub(crate) fn new(tabs: Vec<T>) -> Self {
        assert!(!tabs.is_empty(), "a tab list requires at least one tab");
        Self { tabs, active: 0 }
    }

    pub(crate) fn replace_tabs(&mut self, tabs: Vec<T>) {
        assert!(!tabs.is_empty(), "a tab list requires at least one tab");
        self.tabs = tabs;
        self.active = self.active.min(self.tabs.len() - 1);
    }

    pub(crate) fn tabs(&self) -> &[T] {
        &self.tabs
    }

    pub(crate) fn active_index(&self) -> usize {
        self.active
    }

    pub(crate) fn active_tab(&self) -> &T {
        &self.tabs[self.active]
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> TabListInputOutcome {
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return TabListInputOutcome::Unhandled;
        }
        let previous = self.active;
        match key.code {
            KeyCode::Left | KeyCode::BackTab => {
                self.active = self.active.checked_sub(1).unwrap_or(self.tabs.len() - 1);
            }
            KeyCode::Right | KeyCode::Tab => {
                self.active = (self.active + 1) % self.tabs.len();
            }
            _ => return TabListInputOutcome::Unhandled,
        }
        if self.active == previous {
            TabListInputOutcome::Consumed
        } else {
            TabListInputOutcome::ActiveChanged
        }
    }
}

pub(crate) fn desired_height<T: TabListItem>(tabs: &[T], width: u16) -> u16 {
    tab_lines(tabs, 0, width, Color::Reset)
        .len()
        .min(u16::MAX as usize) as u16
}

pub(crate) fn draw<T: TabListItem>(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &TabListState<T>,
    highlight: Color,
) {
    frame.render_widget(
        Paragraph::new(tab_lines(
            state.tabs(),
            state.active_index(),
            area.width,
            highlight,
        )),
        area,
    );
}

fn tab_lines<T: TabListItem>(
    tabs: &[T],
    active: usize,
    width: u16,
    highlight: Color,
) -> Vec<Line<'static>> {
    if tabs.is_empty() {
        return Vec::new();
    }
    let available_width = usize::from(width.max(1));
    let mut lines = Vec::new();
    let mut spans = Vec::new();
    let mut row_width = 0usize;

    for (index, tab) in tabs.iter().enumerate() {
        let label = tab.tab_label();
        let tab_width = label.width().saturating_add(2);
        let gap = usize::from(!spans.is_empty()) * TAB_GAP;
        if !spans.is_empty()
            && row_width.saturating_add(gap).saturating_add(tab_width) > available_width
        {
            lines.push(Line::from(spans));
            spans = Vec::new();
            row_width = 0;
        }
        if !spans.is_empty() {
            spans.push(Span::raw("  "));
            row_width = row_width.saturating_add(TAB_GAP);
        }
        if index == active {
            spans.push(Span::styled(
                format!(" {label} "),
                Style::default()
                    .fg(Color::Black)
                    .bg(highlight)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                format!(" {label} "),
                Style::default().fg(muted()),
            ));
        }
        row_width = row_width.saturating_add(tab_width);
    }
    if !spans.is_empty() {
        lines.push(Line::from(spans));
    }
    lines
}

#[cfg(test)]
#[path = "tab_list_tests.rs"]
mod tests;
