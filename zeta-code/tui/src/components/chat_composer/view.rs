use super::ChatComposerOverlayView;
use super::ChatComposerPaneKind;
use super::ChatComposerPaneView;
use super::ChatComposerView;
use crate::components::chat_input;
use crate::components::chat_input::ChatInputCursor;
use crate::components::pane;
use crate::components::pane::PanePointerTarget;
use crate::components::suggest;
use crate::render::RenderContext;
use crate::render::Renderable;
use ratatui::Frame;
use ratatui::layout::Rect;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ChatComposerAreas {
    pub(crate) panes: Vec<ChatComposerPaneArea>,
    pub(crate) input: Rect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ChatComposerPaneArea {
    pub(crate) kind: ChatComposerPaneKind,
    pub(crate) area: Rect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ChatComposerPointerTarget {
    PaneTab(usize),
    PaneItem(usize),
    SuggestItem(usize),
}

pub(crate) struct ChatComposerSurface<'a, 'view> {
    pub(crate) overlay_area: Rect,
    pub(crate) view: &'view ChatComposerView<'a>,
    pub(crate) cursor: ChatInputCursor,
}

impl Renderable for ChatComposerSurface<'_, '_> {
    fn desired_height(&self, width: u16) -> u16 {
        let input_height = self.view.input_desired_height(width);
        desired_height(input_height, &pane_sizes(self.view, width))
    }

    fn render(&self, frame: &mut Frame<'_>, area: Rect, context: RenderContext<'_>) {
        let areas = view_areas(area, self.view);
        for (entry, allocation) in self.view.pane_views().into_iter().zip(&areas.panes) {
            debug_assert_eq!(entry.kind(), allocation.kind);
            match entry {
                ChatComposerPaneView::Stacked(view) => {
                    pane::draw(frame, allocation.area, view, context);
                }
            }
        }

        chat_input::draw_chat_input(
            frame,
            areas.input,
            self.view.input(),
            self.view.input_cursor_width(),
            self.view.input_cursor_line(),
            self.view.input_prompt(),
            self.cursor,
            context,
        );

        match self.view.overlay() {
            Some(ChatComposerOverlayView::Suggest(view)) => {
                suggest::draw(frame, self.overlay_area, Some(view), context);
            }
            None => {}
        }
    }
}

pub(crate) fn view_areas(area: Rect, view: &ChatComposerView<'_>) -> ChatComposerAreas {
    let input_height = view.input_desired_height(area.width);
    areas(area, input_height, &pane_sizes(view, area.width))
}

pub(crate) fn pointer_target_at(
    areas: &ChatComposerAreas,
    overlay_area: Rect,
    view: &ChatComposerView<'_>,
    column: u16,
    row: u16,
) -> Option<ChatComposerPointerTarget> {
    for (entry, allocation) in view.pane_views().into_iter().zip(&areas.panes) {
        match entry {
            ChatComposerPaneView::Stacked(view) => {
                if let Some(target) = pane::pointer_target_at(allocation.area, view, column, row) {
                    return Some(match target {
                        PanePointerTarget::Tab(index) => ChatComposerPointerTarget::PaneTab(index),
                        PanePointerTarget::Item(index) => {
                            ChatComposerPointerTarget::PaneItem(index)
                        }
                    });
                }
            }
        }
    }

    let target = match view.overlay()? {
        ChatComposerOverlayView::Suggest(view) => {
            suggest::index_at(overlay_area, Some(view), column, row)
                .map(ChatComposerPointerTarget::SuggestItem)
        }
    };
    target
}

fn pane_sizes(
    view: &ChatComposerView<'_>,
    available_width: u16,
) -> Vec<(ChatComposerPaneKind, u16)> {
    view.pane_views()
        .into_iter()
        .map(|entry| {
            let kind = entry.kind();
            let height = match entry {
                ChatComposerPaneView::Stacked(view) => {
                    pane::view_desired_height(view, available_width)
                }
            };
            (kind, height)
        })
        .collect()
}

pub(crate) fn desired_height(input_height: u16, pane_sizes: &[(ChatComposerPaneKind, u16)]) -> u16 {
    pane_sizes
        .iter()
        .fold(input_height, |height, (_, entry_height)| {
            height.saturating_add(*entry_height)
        })
}

pub(crate) fn areas(
    area: Rect,
    input_desired_height: u16,
    pane_sizes: &[(ChatComposerPaneKind, u16)],
) -> ChatComposerAreas {
    let input_height = input_desired_height.min(area.height);
    let input_y = area
        .y
        .saturating_add(area.height)
        .saturating_sub(input_height);
    let input = Rect {
        y: input_y,
        height: input_height,
        ..area
    };
    let mut next_y = input_y;
    let mut remaining_height = area.height.saturating_sub(input_height);
    let mut allocated = Vec::with_capacity(pane_sizes.len());
    for (kind, desired_height) in pane_sizes {
        let height = (*desired_height).min(remaining_height);
        next_y = next_y.saturating_sub(height);
        remaining_height = remaining_height.saturating_sub(height);
        allocated.push(ChatComposerPaneArea {
            kind: *kind,
            area: Rect {
                y: next_y,
                height,
                ..area
            },
        });
    }

    ChatComposerAreas {
        panes: allocated,
        input,
    }
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
