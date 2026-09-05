use crate::render::InteractionState;
use crate::render::InteractionTarget;
use crate::render::RenderContext;
use crate::render::interaction_style;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use ratatui::Frame;
use ratatui::layout::Rect;
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

    pub(crate) fn select(&mut self, index: usize) -> TabListInputOutcome {
        if index >= self.tabs.len() {
            return TabListInputOutcome::Unhandled;
        }
        let previous = self.active;
        self.active = index;
        if self.active == previous {
            TabListInputOutcome::Consumed
        } else {
            TabListInputOutcome::ActiveChanged
        }
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> TabListInputOutcome {
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return TabListInputOutcome::Unhandled;
        }
        if key.kind != KeyEventKind::Press {
            return if matches!(
                key.code,
                KeyCode::Tab | KeyCode::BackTab | KeyCode::Left | KeyCode::Right
            ) {
                TabListInputOutcome::Consumed
            } else {
                TabListInputOutcome::Unhandled
            };
        }
        let previous = self.active;
        match key.code {
            KeyCode::BackTab | KeyCode::Left => {
                self.active = self.active.checked_sub(1).unwrap_or(self.tabs.len() - 1);
            }
            KeyCode::Tab if key.modifiers == KeyModifiers::SHIFT => {
                self.active = self.active.checked_sub(1).unwrap_or(self.tabs.len() - 1);
            }
            KeyCode::Tab | KeyCode::Right => {
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

impl<T: TabListItem> TabListState<T> {
    pub(crate) fn index_at(&self, area: Rect, column: u16, row: u16) -> Option<usize> {
        if area.width == 0
            || area.height == 0
            || column < area.x
            || column >= area.right()
            || row < area.y
            || row >= area.bottom()
        {
            return None;
        }
        let row = usize::from(row - area.y);
        let column = usize::from(column - area.x);
        tab_positions(&self.tabs, area.width)
            .into_iter()
            .enumerate()
            .find_map(|(index, position)| {
                (position.row == row
                    && column >= position.start
                    && column < position.start.saturating_add(position.width))
                .then_some(index)
            })
    }
}

pub(crate) fn desired_height<T: TabListItem>(tabs: &[T], width: u16) -> u16 {
    tab_positions(tabs, width)
        .last()
        .map(|position| position.row.saturating_add(1))
        .unwrap_or_default()
        .min(u16::MAX as usize) as u16
}

pub(crate) fn draw<T: TabListItem>(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &TabListState<T>,
    focused: bool,
    hovered: Option<usize>,
    pressed: Option<usize>,
    context: RenderContext<'_>,
) {
    frame.render_widget(
        Paragraph::new(tab_lines(
            state.tabs(),
            state.active_index(),
            area.width,
            focused,
            hovered,
            pressed,
            context,
        )),
        area,
    );
}

fn tab_lines<T: TabListItem>(
    tabs: &[T],
    active: usize,
    width: u16,
    focused: bool,
    hovered: Option<usize>,
    pressed: Option<usize>,
    context: RenderContext<'_>,
) -> Vec<Line<'static>> {
    if tabs.is_empty() {
        return Vec::new();
    }
    let positions = tab_positions(tabs, width);
    let row_count = positions
        .last()
        .map(|position| position.row.saturating_add(1))
        .unwrap_or_default();
    let mut lines = (0..row_count)
        .map(|_| Vec::new())
        .collect::<Vec<Vec<Span<'static>>>>();
    let mut row_widths = vec![0usize; row_count];

    for (index, tab) in tabs.iter().enumerate() {
        let position = positions[index];
        let spans = &mut lines[position.row];
        let row_width = &mut row_widths[position.row];
        if *row_width < position.start {
            spans.push(Span::raw(" ".repeat(position.start - *row_width)));
            *row_width = position.start;
        }
        let target = if index == active {
            InteractionTarget::Active
        } else {
            InteractionTarget::Rest
        };
        let state = InteractionState {
            target,
            selected: focused && index == active,
            hovered: target == InteractionTarget::Rest && hovered == Some(index),
            pressed: pressed == Some(index),
        };
        let mut style = interaction_style(context, state);
        if target == InteractionTarget::Rest && !state.selected && !state.pressed {
            style = Style::default().fg(if state.hovered {
                context.foreground()
            } else {
                context.muted()
            });
            if state.hovered {
                style = style.add_modifier(Modifier::UNDERLINED);
            }
        }
        spans.push(Span::styled(format!(" {} ", tab.tab_label()), style));
        *row_width = position.start.saturating_add(position.width);
    }
    lines.into_iter().map(Line::from).collect()
}

#[derive(Clone, Copy)]
struct TabPosition {
    row: usize,
    start: usize,
    width: usize,
}

fn tab_positions<T: TabListItem>(tabs: &[T], width: u16) -> Vec<TabPosition> {
    let available_width = usize::from(width.max(1));
    let mut positions = Vec::with_capacity(tabs.len());
    let mut row = 0usize;
    let mut row_width = 0usize;

    for tab in tabs {
        let tab_width = tab.tab_label().width().saturating_add(2);
        let gap = usize::from(row_width > 0) * TAB_GAP;
        if row_width > 0
            && row_width.saturating_add(gap).saturating_add(tab_width) > available_width
        {
            row = row.saturating_add(1);
            row_width = 0;
        }
        let start = row_width.saturating_add(usize::from(row_width > 0) * TAB_GAP);
        positions.push(TabPosition {
            row,
            start,
            width: tab_width,
        });
        row_width = start.saturating_add(tab_width);
    }
    positions
}

#[cfg(test)]
#[path = "tab_list_tests.rs"]
mod tests;
