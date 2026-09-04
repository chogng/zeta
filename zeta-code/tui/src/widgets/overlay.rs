//! Read-only detail layer rendered without changing the current screen layout.

use crate::render::RenderContext;
use crate::render::bottom_anchored_area;
use crate::render::horizontal_margin;
use crate::widgets::detail_list;
use crate::widgets::detail_list::DetailList;
use crate::widgets::key_hint;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Block;
use ratatui::widgets::Clear;

const TITLE_ROWS: u16 = 1;
const BOTTOM_GAP_ROWS: u16 = 3;
const KEY_HINT_ROWS: u16 = 1;

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

    pub(crate) fn handle_key(&mut self, key: KeyEvent, available: Rect) -> OverlayInputOutcome {
        if key.kind != KeyEventKind::Press {
            return OverlayInputOutcome::Consumed;
        }
        let max_scroll = overlay_layout(available, &self.detail).max_scroll;
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Esc) => OverlayInputOutcome::Dismiss,
            (KeyModifiers::NONE, KeyCode::Up) => {
                self.scroll = self.scroll.saturating_sub(1);
                OverlayInputOutcome::Consumed
            }
            (KeyModifiers::NONE, KeyCode::Down) => {
                self.scroll = self.scroll.saturating_add(1).min(max_scroll);
                OverlayInputOutcome::Consumed
            }
            (KeyModifiers::NONE, KeyCode::PageUp) => {
                self.scroll = self.scroll.saturating_sub(10);
                OverlayInputOutcome::Consumed
            }
            (KeyModifiers::NONE, KeyCode::PageDown) => {
                self.scroll = self.scroll.saturating_add(10).min(max_scroll);
                OverlayInputOutcome::Consumed
            }
            (KeyModifiers::CONTROL, KeyCode::Home) => {
                self.scroll = 0;
                OverlayInputOutcome::Consumed
            }
            (KeyModifiers::CONTROL, KeyCode::End) => {
                self.scroll = max_scroll;
                OverlayInputOutcome::Consumed
            }
            _ => OverlayInputOutcome::Consumed,
        }
    }
}

#[derive(Clone, Copy)]
struct DetailOverlayLayout {
    surface: Rect,
    body: Rect,
    hints: Rect,
    max_scroll: u16,
}

fn overlay_layout(available: Rect, detail: &DetailList) -> DetailOverlayLayout {
    let content_width = horizontal_margin(available, 2).width;
    let content_rows = u16::try_from(detail.content_height(content_width)).unwrap_or(u16::MAX);
    let desired_body_rows = TITLE_ROWS
        .saturating_add(content_rows)
        .saturating_add(BOTTOM_GAP_ROWS);
    let surface_rows = desired_body_rows
        .saturating_add(KEY_HINT_ROWS)
        .min(available.height);
    let surface = bottom_anchored_area(available, surface_rows);
    let hint_rows = KEY_HINT_ROWS.min(surface.height);
    let body_rows = surface.height.saturating_sub(hint_rows);
    let body = Rect {
        height: body_rows,
        ..surface
    };
    let hints = Rect {
        y: surface.y.saturating_add(body_rows),
        height: hint_rows,
        ..surface
    };
    let visible_content_rows = body_rows.saturating_sub(TITLE_ROWS);
    DetailOverlayLayout {
        surface,
        body,
        hints,
        max_scroll: content_rows.saturating_sub(visible_content_rows),
    }
}

pub(crate) fn draw(
    frame: &mut Frame<'_>,
    available: Rect,
    state: &DetailOverlay,
    context: RenderContext<'_>,
) {
    let layout = overlay_layout(available, &state.detail);
    frame.render_widget(Clear, layout.surface);
    frame.render_widget(
        Block::default().style(Style::default().bg(context.overlay_background())),
        layout.surface,
    );
    detail_list::draw_scrolled(
        frame,
        layout.body,
        &state.detail,
        state.scroll.min(layout.max_scroll),
        context,
    );
    key_hint::draw(frame, layout.hints, "Esc to close", context);
}

#[cfg(test)]
#[path = "overlay_tests.rs"]
mod tests;
