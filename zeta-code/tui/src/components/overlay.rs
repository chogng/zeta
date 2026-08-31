//! Read-only detail layer rendered without changing the current screen layout.

use crate::components::detail_list;
use crate::components::detail_list::DetailList;
use crate::components::key_hint;
use crate::render::RenderContext;
use crate::render::bottom_anchored_area;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Block;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OverlayInputOutcome {
    Consumed,
    Dismiss,
}

#[derive(Debug)]
pub(crate) struct DetailOverlay {
    detail: DetailList,
    scroll: u16,
}

impl DetailOverlay {
    pub(crate) fn new(detail: DetailList) -> Self {
        Self { detail, scroll: 0 }
    }

    #[cfg(test)]
    pub(crate) fn title(&self) -> &str {
        self.detail.title()
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> OverlayInputOutcome {
        if key.kind != KeyEventKind::Press {
            return OverlayInputOutcome::Consumed;
        }
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Esc) => OverlayInputOutcome::Dismiss,
            (KeyModifiers::NONE, KeyCode::Up) => {
                self.scroll = self.scroll.saturating_sub(1);
                OverlayInputOutcome::Consumed
            }
            (KeyModifiers::NONE, KeyCode::Down) => {
                self.scroll = self.scroll.saturating_add(1).min(self.max_scroll());
                OverlayInputOutcome::Consumed
            }
            (KeyModifiers::NONE, KeyCode::PageUp) => {
                self.scroll = self.scroll.saturating_sub(10);
                OverlayInputOutcome::Consumed
            }
            (KeyModifiers::NONE, KeyCode::PageDown) => {
                self.scroll = self.scroll.saturating_add(10).min(self.max_scroll());
                OverlayInputOutcome::Consumed
            }
            (KeyModifiers::CONTROL, KeyCode::Home) => {
                self.scroll = 0;
                OverlayInputOutcome::Consumed
            }
            (KeyModifiers::CONTROL, KeyCode::End) => {
                self.scroll = self.max_scroll();
                OverlayInputOutcome::Consumed
            }
            _ => OverlayInputOutcome::Consumed,
        }
    }

    fn max_scroll(&self) -> u16 {
        self.detail.desired_height().saturating_sub(1)
    }
}

pub(crate) fn draw(
    frame: &mut Frame<'_>,
    available: Rect,
    state: &DetailOverlay,
    context: RenderContext<'_>,
) {
    let height = state
        .detail
        .desired_height()
        .saturating_add(2)
        .min(available.height);
    let width = available.width.min(100);
    let centered = Rect {
        x: available
            .x
            .saturating_add(available.width.saturating_sub(width) / 2),
        width,
        ..available
    };
    let area = bottom_anchored_area(centered, height);
    frame.render_widget(
        Block::default().style(Style::default().bg(context.overlay_background())),
        area,
    );
    let key_rows = u16::from(area.height > 0);
    let body = Rect {
        height: area.height.saturating_sub(key_rows),
        ..area
    };
    let hints = Rect {
        y: area.y.saturating_add(area.height.saturating_sub(key_rows)),
        height: key_rows,
        ..area
    };
    let visible_scroll = state
        .scroll
        .min(state.detail.desired_height().saturating_sub(body.height));
    detail_list::draw_scrolled(frame, body, &state.detail, visible_scroll, context);
    key_hint::draw(frame, hints, "Esc to close", context);
}

#[cfg(test)]
#[path = "overlay_tests.rs"]
mod tests;
