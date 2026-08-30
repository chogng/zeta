use ratatui::buffer::Buffer;
use ratatui::layout::Position;
use ratatui::style::Modifier;
use unicode_width::UnicodeWidthStr;

/// Declares whether the active TUI surface leaves pointer input to the terminal or receives it for
/// screen selection and click handling.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum MouseMode {
    #[default]
    TerminalSelection,
    TuiCapture,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ScreenSelectionOutcome {
    Click(Position),
    Copy(ScreenSelectionRange),
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
    }

    pub(crate) fn finish(&mut self, position: Position) -> Option<ScreenSelectionOutcome> {
        let anchor = self.anchor?;
        self.focus = Some(position);
        self.dragging |= position != anchor;
        if self.dragging {
            Some(ScreenSelectionOutcome::Copy(ScreenSelectionRange::new(
                anchor, position,
            )))
        } else {
            self.clear();
            Some(ScreenSelectionOutcome::Click(position))
        }
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

    pub(crate) fn draw(&self, buffer: &mut Buffer) {
        let Some(range) = self.range() else {
            return;
        };
        let area = buffer.area;
        for row in area.y..area.bottom() {
            for column in area.x..area.right() {
                let position = Position::new(column, row);
                if range.contains(position)
                    && let Some(cell) = buffer.cell_mut(position)
                {
                    cell.modifier.insert(Modifier::REVERSED);
                }
            }
        }
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
        lines.push(line.trim_end().to_owned());
    }
    let text = lines.join("\n");
    (!text.is_empty()).then_some(text)
}

const fn position_index(position: Position) -> u32 {
    (position.y as u32) << 16 | position.x as u32
}

#[cfg(test)]
#[path = "mouse_tests.rs"]
mod tests;
