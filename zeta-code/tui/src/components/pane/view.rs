use super::PaneBodyView;
use super::PaneView;
use crate::components::key_capture;
use crate::components::list_selection;
use crate::components::text_prompt;
use crate::render::InteractionAttention;
use crate::render::InteractionState;
use crate::render::InteractionTarget;
use crate::render::RenderContext;
use crate::render::interaction_style;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;

const TITLE_BAR_HEIGHT: u16 = 1;

pub(crate) struct PaneAreas {
    pub(crate) body: Rect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PanePointerTarget {
    Tab(usize),
    Search,
    Item(usize),
}

pub(crate) fn view_desired_height(view: PaneView<'_>, available_width: u16) -> u16 {
    let body_height = match view.body() {
        PaneBodyView::KeyCapture(body) => body.desired_height(),
        PaneBodyView::ListSelection(body) => body.desired_height(available_width),
        PaneBodyView::TextPrompt(body) => body.desired_height(),
    };
    desired_height(body_height)
}

pub(crate) fn draw(
    frame: &mut Frame<'_>,
    area: Rect,
    view: PaneView<'_>,
    hovered: Option<PanePointerTarget>,
    pressed: Option<PanePointerTarget>,
    context: RenderContext<'_>,
) {
    let pane_areas = areas(area);
    let presentation_highlight = match view.body() {
        PaneBodyView::ListSelection(body) => body
            .presentation_highlight()
            .unwrap_or_else(|| context.focus()),
        PaneBodyView::KeyCapture(_) | PaneBodyView::TextPrompt(_) => context.focus(),
    };
    let title_style = interaction_style(
        context,
        InteractionState {
            target: InteractionTarget::Active,
            attention: InteractionAttention::None,
        },
    );
    frame.render_widget(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(presentation_highlight))
            .title(Line::from(vec![
                Span::styled("─", Style::default().fg(presentation_highlight)),
                Span::styled(format!(" {} ", view.title()), title_style),
            ])),
        area,
    );
    match view.body() {
        PaneBodyView::KeyCapture(body) => key_capture::draw(frame, pane_areas.body, body, context),
        PaneBodyView::ListSelection(body) => list_selection::draw_with_pointer(
            frame,
            pane_areas.body,
            body,
            tab_index(hovered),
            tab_index(pressed),
            hovered == Some(PanePointerTarget::Search),
            pressed == Some(PanePointerTarget::Search),
            item_index(hovered),
            item_index(pressed),
            context,
        ),
        PaneBodyView::TextPrompt(body) => text_prompt::draw(frame, pane_areas.body, body, context),
    }
}

fn tab_index(target: Option<PanePointerTarget>) -> Option<usize> {
    match target {
        Some(PanePointerTarget::Tab(index)) => Some(index),
        Some(PanePointerTarget::Search | PanePointerTarget::Item(_)) | None => None,
    }
}

fn item_index(target: Option<PanePointerTarget>) -> Option<usize> {
    match target {
        Some(PanePointerTarget::Item(index)) => Some(index),
        Some(PanePointerTarget::Tab(_) | PanePointerTarget::Search) | None => None,
    }
}

pub(crate) fn pointer_target_at(
    area: Rect,
    view: PaneView<'_>,
    column: u16,
    row: u16,
) -> Option<PanePointerTarget> {
    let PaneBodyView::ListSelection(body) = view.body() else {
        return None;
    };
    let body_area = areas(area).body;
    body.tab_index_at(body_area, column, row)
        .map(PanePointerTarget::Tab)
        .or_else(|| {
            body.search_contains(body_area, column, row)
                .then_some(PanePointerTarget::Search)
        })
        .or_else(|| {
            body.item_index_at(body_area, column, row)
                .map(PanePointerTarget::Item)
        })
}

pub(crate) fn desired_height(body_height: u16) -> u16 {
    body_height.saturating_add(TITLE_BAR_HEIGHT)
}

pub(crate) fn areas(area: Rect) -> PaneAreas {
    let title_bar_height = TITLE_BAR_HEIGHT.min(area.height);
    PaneAreas {
        body: Rect {
            y: area.y.saturating_add(title_bar_height),
            height: area.height.saturating_sub(title_bar_height),
            ..area
        },
    }
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
