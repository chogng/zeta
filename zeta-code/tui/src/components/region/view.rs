use super::RegionView;
use crate::components::key_capture;
use crate::components::list_selection;
use crate::components::text_prompt;
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ComposerModePointerTarget {
    Tab(usize),
    Search,
    Item(usize),
}

pub(crate) fn view_desired_height(view: RegionView<'_>, available_width: u16) -> u16 {
    let body_height = match view {
        RegionView::KeyCapture(body) => body.desired_height(),
        RegionView::ListSelection(body) => body.desired_height(available_width),
        RegionView::TextPrompt(body) => body.desired_height(),
    };
    body_height.saturating_add(TITLE_BAR_HEIGHT)
}

pub(crate) fn draw(
    frame: &mut Frame<'_>,
    area: Rect,
    view: RegionView<'_>,
    hovered: Option<ComposerModePointerTarget>,
    pressed: Option<ComposerModePointerTarget>,
    context: RenderContext<'_>,
) {
    let body_area = body_area(area);
    let presentation_focus = match view {
        RegionView::ListSelection(body) => {
            body.presentation_focus().unwrap_or_else(|| context.focus())
        }
        RegionView::KeyCapture(_) | RegionView::TextPrompt(_) => context.focus(),
    };
    let title_style = interaction_style(
        context,
        InteractionState {
            target: InteractionTarget::Active,
            selected: false,
            hovered: false,
            pressed: false,
        },
    );
    frame.render_widget(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(presentation_focus))
            .title(Line::from(vec![
                Span::styled("─", Style::default().fg(presentation_focus)),
                Span::styled(format!(" {} ", view.title()), title_style),
            ])),
        area,
    );
    match view {
        RegionView::KeyCapture(body) => key_capture::draw(frame, body_area, body, context),
        RegionView::ListSelection(body) => list_selection::draw_with_pointer(
            frame,
            body_area,
            body,
            tab_index(hovered),
            tab_index(pressed),
            hovered == Some(ComposerModePointerTarget::Search),
            pressed == Some(ComposerModePointerTarget::Search),
            item_index(hovered),
            item_index(pressed),
            context,
        ),
        RegionView::TextPrompt(body) => text_prompt::draw(frame, body_area, body, context),
    }
}

pub(crate) fn pointer_target_at(
    area: Rect,
    view: RegionView<'_>,
    column: u16,
    row: u16,
) -> Option<ComposerModePointerTarget> {
    let RegionView::ListSelection(body) = view else {
        return None;
    };
    let body_area = body_area(area);
    body.tab_index_at(body_area, column, row)
        .map(ComposerModePointerTarget::Tab)
        .or_else(|| {
            body.search_contains(body_area, column, row)
                .then_some(ComposerModePointerTarget::Search)
        })
        .or_else(|| {
            body.item_index_at(body_area, column, row)
                .map(ComposerModePointerTarget::Item)
        })
}

fn tab_index(target: Option<ComposerModePointerTarget>) -> Option<usize> {
    match target {
        Some(ComposerModePointerTarget::Tab(index)) => Some(index),
        Some(ComposerModePointerTarget::Search | ComposerModePointerTarget::Item(_)) | None => None,
    }
}

fn item_index(target: Option<ComposerModePointerTarget>) -> Option<usize> {
    match target {
        Some(ComposerModePointerTarget::Item(index)) => Some(index),
        Some(ComposerModePointerTarget::Tab(_) | ComposerModePointerTarget::Search) | None => None,
    }
}

fn body_area(area: Rect) -> Rect {
    let title_bar_height = TITLE_BAR_HEIGHT.min(area.height);
    Rect {
        y: area.y.saturating_add(title_bar_height),
        height: area.height.saturating_sub(title_bar_height),
        ..area
    }
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
