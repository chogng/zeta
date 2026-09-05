//! Keys shared by focused lists and read-only surfaces, never by text editors.

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Navigation {
    Previous,
    Next,
    PagePrevious,
    PageNext,
    First,
    Last,
}

impl Navigation {
    pub(crate) fn from_key(key: KeyEvent) -> Option<Self> {
        if key.kind == KeyEventKind::Release {
            return None;
        }
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Up | KeyCode::Char('k')) => Some(Self::Previous),
            (KeyModifiers::NONE, KeyCode::Down | KeyCode::Char('j')) => Some(Self::Next),
            (KeyModifiers::NONE, KeyCode::PageUp) => Some(Self::PagePrevious),
            (KeyModifiers::NONE, KeyCode::PageDown) => Some(Self::PageNext),
            (KeyModifiers::NONE | KeyModifiers::CONTROL, KeyCode::Home) => Some(Self::First),
            (KeyModifiers::NONE | KeyModifiers::CONTROL, KeyCode::End) => Some(Self::Last),
            _ => None,
        }
    }

    pub(crate) fn offset(self, current: usize, last: usize, page_rows: usize) -> usize {
        let current = current.min(last);
        match self {
            Self::Previous => current.saturating_sub(1),
            Self::Next => current.saturating_add(1).min(last),
            Self::PagePrevious => current.saturating_sub(page_rows.max(1)),
            Self::PageNext => current.saturating_add(page_rows.max(1)).min(last),
            Self::First => 0,
            Self::Last => last,
        }
    }
}

#[cfg(test)]
#[path = "navigation_tests.rs"]
mod tests;
