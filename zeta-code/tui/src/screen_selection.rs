use crate::render::RenderContext;
use ratatui::buffer::Buffer;
use ratatui::layout::Position;
use ratatui::style::Modifier;
use std::time::Duration;
use std::time::Instant;
use unicode_width::UnicodeWidthStr;

const MULTI_CLICK_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ScreenSelectionOutcome {
    Click {
        position: Position,
        count: ClickCount,
    },
    Copy(ScreenSelectionRange),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ClickCount {
    Single,
    Double,
    Triple,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ScreenSelectionRange {
    start: Position,
    end: Position,
}

impl ScreenSelectionRange {
    fn new(anchor: Position, focus: Position) -> Self {
        let (start, end) = if position_index(anchor) <= position_index(focus) {
            (anchor, focus)
        } else {
            (focus, anchor)
        };
        Self { start, end }
    }

    fn contains(self, position: Position) -> bool {
        position_index(self.start) <= position_index(position)
            && position_index(position) <= position_index(self.end)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ScreenSelection {
    anchor: Option<Position>,
    focus: Option<Position>,
    dragging: bool,
    click_sequence: Option<ClickSequence>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ClickSequence {
    position: Position,
    count: ClickCount,
    completed_at: Instant,
}

impl ScreenSelection {
    pub(crate) fn begin(&mut self, position: Position) {
        self.anchor = Some(position);
        self.focus = Some(position);
        self.dragging = false;
    }

    pub(crate) fn drag(&mut self, position: Position) {
        let Some(anchor) = self.anchor else {
            return;
        };
        self.focus = Some(position);
        self.dragging |= position != anchor;
        if self.dragging {
            self.click_sequence = None;
        }
    }

    pub(crate) fn finish(
        &mut self,
        position: Position,
        now: Instant,
    ) -> Option<ScreenSelectionOutcome> {
        let anchor = self.anchor?;
        self.focus = Some(position);
        self.dragging |= position != anchor;
        if self.dragging {
            self.click_sequence = None;
            Some(ScreenSelectionOutcome::Copy(ScreenSelectionRange::new(
                anchor, position,
            )))
        } else {
            self.anchor = None;
            self.focus = None;
            let count = self.next_click_count(position, now);
            self.click_sequence = Some(ClickSequence {
                position,
                count,
                completed_at: now,
            });
            Some(ScreenSelectionOutcome::Click { position, count })
        }
    }

    fn next_click_count(&self, position: Position, now: Instant) -> ClickCount {
        let Some(previous) = self.click_sequence else {
            return ClickCount::Single;
        };
        let close_position =
            previous.position.y == position.y && previous.position.x.abs_diff(position.x) <= 1;
        let within_interval = now
            .checked_duration_since(previous.completed_at)
            .is_some_and(|elapsed| elapsed <= MULTI_CLICK_INTERVAL);
        if !close_position || !within_interval {
            return ClickCount::Single;
        }
        match previous.count {
            ClickCount::Single => ClickCount::Double,
            ClickCount::Double => ClickCount::Triple,
            ClickCount::Triple => ClickCount::Single,
        }
    }

    pub(crate) fn select(&mut self, range: ScreenSelectionRange) {
        self.anchor = Some(range.start);
        self.focus = Some(range.end);
        self.dragging = true;
    }

    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn range(&self) -> Option<ScreenSelectionRange> {
        self.dragging.then(|| {
            ScreenSelectionRange::new(
                self.anchor.expect("a dragging selection has an anchor"),
                self.focus.expect("a dragging selection has a focus"),
            )
        })
    }

    pub(crate) fn draw(&self, buffer: &mut Buffer, context: RenderContext<'_>) {
        let Some(range) = self.range() else {
            return;
        };
        let foreground = context.screen_selection_foreground();
        let background = context.screen_selection_background();
        let area = buffer.area;
        for row in area.y..area.bottom() {
            for column in area.x..area.right() {
                let position = Position::new(column, row);
                if range.contains(position)
                    && let Some(cell) = buffer.cell_mut(position)
                {
                    cell.fg = foreground;
                    cell.bg = background;
                    cell.modifier.remove(Modifier::REVERSED);
                }
            }
        }
    }
}

pub(crate) fn token_range_at(buffer: &Buffer, position: Position) -> Option<ScreenSelectionRange> {
    let area = buffer.area;
    if !area.contains(position) {
        return None;
    }
    let units = row_units(buffer, position.y);
    let selected = units
        .iter()
        .position(|unit| unit.start <= position.x && position.x <= unit.end)?;
    let class = units[selected].class;
    let mut first = selected;
    while first > 0 && units[first - 1].class == class {
        first -= 1;
    }
    let mut last = selected;
    while last + 1 < units.len() && units[last + 1].class == class {
        last += 1;
    }
    let range = ScreenSelectionRange::new(
        Position::new(units[first].start, position.y),
        Position::new(units[last].end, position.y),
    );
    text_in_range(buffer, range).map(|_| range)
}

pub(crate) fn line_range_at(buffer: &Buffer, position: Position) -> Option<ScreenSelectionRange> {
    let area = buffer.area;
    if !area.contains(position) || area.is_empty() {
        return None;
    }
    let range = ScreenSelectionRange::new(
        Position::new(area.x, position.y),
        Position::new(area.right().saturating_sub(1), position.y),
    );
    text_in_range(buffer, range).map(|_| range)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CellClass {
    Whitespace,
    Word,
    Symbol,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RowUnit {
    start: u16,
    end: u16,
    class: CellClass,
}

fn row_units(buffer: &Buffer, row: u16) -> Vec<RowUnit> {
    let area = buffer.area;
    let mut units = Vec::with_capacity(usize::from(area.width));
    let mut column = area.x;
    while column < area.right() {
        let Some(cell) = buffer.cell(Position::new(column, row)) else {
            column = column.saturating_add(1);
            continue;
        };
        let symbol = cell.symbol();
        let width = u16::try_from(symbol.width().max(1)).unwrap_or(u16::MAX);
        let end = column
            .saturating_add(width.saturating_sub(1))
            .min(area.right().saturating_sub(1));
        units.push(RowUnit {
            start: column,
            end,
            class: cell_class(symbol),
        });
        column = end.saturating_add(1);
    }
    units
}

fn cell_class(symbol: &str) -> CellClass {
    if symbol.chars().all(char::is_whitespace) {
        CellClass::Whitespace
    } else if symbol
        .chars()
        .any(|character| character.is_alphanumeric() || character == '_')
    {
        CellClass::Word
    } else {
        CellClass::Symbol
    }
}

pub(crate) fn text_in_range(buffer: &Buffer, range: ScreenSelectionRange) -> Option<String> {
    let area = buffer.area;
    if area.is_empty() {
        return None;
    }
    let top = range.start.y.max(area.y);
    let bottom = range.end.y.min(area.bottom().saturating_sub(1));
    if top > bottom {
        return None;
    }

    let mut lines = Vec::new();
    for row in top..=bottom {
        let left = if row == range.start.y {
            range.start.x.max(area.x)
        } else {
            area.x
        };
        let right = if row == range.end.y {
            range.end.x.min(area.right().saturating_sub(1))
        } else {
            area.right().saturating_sub(1)
        };
        if left > right {
            lines.push(String::new());
            continue;
        }

        let mut line = String::new();
        let mut continuation_cells = 0usize;
        for column in area.x..=right {
            let Some(cell) = buffer.cell(Position::new(column, row)) else {
                continue;
            };
            if continuation_cells > 0 {
                continuation_cells -= 1;
                continue;
            }
            continuation_cells = cell.symbol().width().saturating_sub(1);
            if column >= left {
                line.push_str(cell.symbol());
            }
        }
        if right == area.right().saturating_sub(1) {
            line.truncate(line.trim_end().len());
        }
        lines.push(line);
    }
    let text = lines.join("\n");
    (!text.is_empty()).then_some(text)
}

const fn position_index(position: Position) -> u32 {
    (position.y as u32) << 16 | position.x as u32
}

#[cfg(test)]
#[path = "screen_selection_tests.rs"]
mod tests;
