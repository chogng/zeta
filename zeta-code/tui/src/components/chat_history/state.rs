use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

const SCROLL_STEP: usize = 5;

#[derive(Debug, Default)]
pub(crate) struct ChatHistoryScroll {
    rows_from_bottom: usize,
}

impl ChatHistoryScroll {
    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> bool {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::PageUp) => {
                self.rows_from_bottom = self.rows_from_bottom.saturating_add(SCROLL_STEP);
                true
            }
            (KeyModifiers::NONE, KeyCode::PageDown) => {
                self.rows_from_bottom = self.rows_from_bottom.saturating_sub(SCROLL_STEP);
                true
            }
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

    pub(crate) fn paragraph_offset(&self, bottom_offset: usize) -> u16 {
        bottom_offset
            .saturating_sub(self.rows_from_bottom)
            .min(u16::MAX as usize) as u16
    }

    pub(crate) fn follow_latest(&mut self) {
        self.rows_from_bottom = 0;
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
