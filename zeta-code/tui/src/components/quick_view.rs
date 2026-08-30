//! Generic read-only overlay that does not change the normal page layout.

use crate::components::detail_list;
use crate::components::detail_list::DetailList;
use crate::components::key_hint_bar;
use crate::components::pane::PaneSpec;
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

#[derive(Debug)]
pub(crate) struct QuickViewState {
    detail: DetailList,
    key_hints: String,
    scroll: u16,
}

impl QuickViewState {
    pub(crate) fn new(spec: PaneSpec<DetailList>) -> Self {
        let (detail, key_hints) = spec.into_parts();
        Self {
            detail,
            key_hints,
            scroll: 0,
        }
    }

    #[cfg(test)]
    pub(crate) fn title(&self) -> &str {
        self.detail.title()
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.kind != KeyEventKind::Press {
            return false;
        }
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Up) => {
                self.scroll = self.scroll.saturating_sub(1);
                true
            }
            (KeyModifiers::NONE, KeyCode::Down) => {
                self.scroll = self.scroll.saturating_add(1).min(self.max_scroll());
                true
            }
            (KeyModifiers::NONE, KeyCode::PageUp) => {
                self.scroll = self.scroll.saturating_sub(10);
                true
            }
            (KeyModifiers::NONE, KeyCode::PageDown) => {
                self.scroll = self.scroll.saturating_add(10).min(self.max_scroll());
                true
            }
            (KeyModifiers::CONTROL, KeyCode::Home) => {
                self.scroll = 0;
                true
            }
            (KeyModifiers::CONTROL, KeyCode::End) => {
                self.scroll = self.max_scroll();
                true
            }
            _ => false,
        }
    }

    fn max_scroll(&self) -> u16 {
        self.detail.desired_height().saturating_sub(1)
    }
}

pub(crate) fn draw(
    frame: &mut Frame<'_>,
    available: Rect,
    state: &QuickViewState,
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
        Block::default().style(Style::default().bg(context.quick_view_background())),
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
    key_hint_bar::draw(frame, hints, &state.key_hints, context);
}

#[cfg(test)]
#[path = "quick_view_tests.rs"]
mod tests;
