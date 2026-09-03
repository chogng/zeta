use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

const SCROLL_STEP: usize = 5;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TranscriptScrollDirection {
    Up,
    Down,
}

#[derive(Debug, Default)]
pub(crate) struct ChatHistoryScroll {
    rows_from_bottom: usize,
}

impl ChatHistoryScroll {
    pub(crate) fn scroll(&mut self, direction: TranscriptScrollDirection) -> bool {
        let previous = self.rows_from_bottom;
        self.rows_from_bottom = match direction {
            TranscriptScrollDirection::Up => self.rows_from_bottom.saturating_add(SCROLL_STEP),
            TranscriptScrollDirection::Down => self.rows_from_bottom.saturating_sub(SCROLL_STEP),
        };
        self.rows_from_bottom != previous
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> bool {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::PageUp) => self.scroll(TranscriptScrollDirection::Up),
            (KeyModifiers::NONE, KeyCode::PageDown) => self.scroll(TranscriptScrollDirection::Down),
            (KeyModifiers::CONTROL, KeyCode::Home) => {
                self.rows_from_bottom = usize::MAX;
                true
            }
            (KeyModifiers::CONTROL, KeyCode::End) => {
                self.rows_from_bottom = 0;
                true
            }
            _ => false,
        }
    }

    pub(crate) fn paragraph_offset(&self, bottom_offset: usize) -> usize {
        bottom_offset.saturating_sub(self.rows_from_bottom)
    }

    pub(crate) fn is_scrolled(&self, bottom_offset: usize) -> bool {
        self.paragraph_offset(bottom_offset) < bottom_offset
    }

    pub(crate) fn follow_latest(&mut self) {
        self.rows_from_bottom = 0;
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
