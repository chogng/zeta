use super::ChatComposerPaneKind;
use super::ChatComposerPaneView;
use super::ChatComposerView;
use crate::components::chat_input;
use crate::components::chat_input::ChatInputCursor;
use crate::components::pane;
use crate::components::pane::PanePointerTarget;
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
    PaneSearch,
    PaneItem(usize),
    CompletionItem(usize),
}

pub(crate) struct ChatComposerSurface<'a, 'view> {
    pub(crate) overlay_area: Rect,
    pub(crate) view: &'view ChatComposerView<'a>,
    pub(crate) cursor: ChatInputCursor,
    pub(crate) hovered: Option<ChatComposerPointerTarget>,
    pub(crate) pressed: Option<ChatComposerPointerTarget>,
}

impl Renderable for ChatComposerSurface<'_, '_> {
    fn desired_height(&self, width: u16, _context: RenderContext<'_>) -> u16 {
        let input_height = self.view.input_desired_height(width);
        desired_height(input_height, &pane_sizes(self.view, width))
    }

    fn render(&self, frame: &mut Frame<'_>, area: Rect, context: RenderContext<'_>) {
        let areas = view_areas(area, self.view);
        for (entry, allocation) in self.view.pane_views().into_iter().zip(&areas.panes) {
            debug_assert_eq!(entry.kind(), allocation.kind);
            match entry {
                ChatComposerPaneView::Stacked(view) => {
                    let hovered = match self.hovered {
                        Some(ChatComposerPointerTarget::PaneTab(index)) => {
                            Some(PanePointerTarget::Tab(index))
                        }
                        Some(ChatComposerPointerTarget::PaneItem(index)) => {
                            Some(PanePointerTarget::Item(index))
                        }
                        Some(ChatComposerPointerTarget::PaneSearch) => {
                            Some(PanePointerTarget::Search)
                        }
                        Some(ChatComposerPointerTarget::CompletionItem(_)) | None => None,
                    };
                    let pressed = match self.pressed {
                        Some(ChatComposerPointerTarget::PaneTab(index)) => {
                            Some(PanePointerTarget::Tab(index))
                        }
                        Some(ChatComposerPointerTarget::PaneItem(index)) => {
                            Some(PanePointerTarget::Item(index))
                        }
                        Some(ChatComposerPointerTarget::PaneSearch) => {
                            Some(PanePointerTarget::Search)
                        }
                        Some(ChatComposerPointerTarget::CompletionItem(_)) | None => None,
                    };
                    pane::draw(frame, allocation.area, view, hovered, pressed, context);
                }
            }
        }

        if !areas.input.is_empty() {
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
        }

        chat_input::draw_completion(
            frame,
            self.overlay_area,
            self.view.input_completion(),
            match self.hovered {
                Some(ChatComposerPointerTarget::CompletionItem(index)) => Some(index),
                _ => None,
            },
            match self.pressed {
                Some(ChatComposerPointerTarget::CompletionItem(index)) => Some(index),
                _ => None,
            },
            context,
        );
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
                        PanePointerTarget::Search => ChatComposerPointerTarget::PaneSearch,
                        PanePointerTarget::Item(index) => {
                            ChatComposerPointerTarget::PaneItem(index)
                        }
                    });
                }
            }
        }
    }

    chat_input::completion_index_at(overlay_area, view.input_completion(), column, row)
        .map(ChatComposerPointerTarget::CompletionItem)
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
    let input_height = if pane_sizes.is_empty() {
        input_desired_height.min(area.height)
    } else {
        0
    };
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
    for (index, (kind, desired_height)) in pane_sizes.iter().enumerate() {
        let absorbed_input_height = if index + 1 == pane_sizes.len() {
            input_desired_height
        } else {
            0
        };
        let height = desired_height
            .saturating_add(absorbed_input_height)
            .min(remaining_height);
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
