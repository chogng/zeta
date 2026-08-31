use super::ChatComposerView;
use crate::components::chat_input;
use crate::components::chat_input::ChatInputCursor;
use crate::render::RenderContext;
use crate::render::Renderable;
use ratatui::Frame;
use ratatui::layout::Rect;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ChatComposerPointerTarget {
    CompletionItem(usize),
}

pub(crate) struct ChatComposerSurface<'a, 'view> {
    pub(crate) view: &'view ChatComposerView<'a>,
    pub(crate) cursor: ChatInputCursor,
}

impl Renderable for ChatComposerSurface<'_, '_> {
    fn desired_height(&self, width: u16, _context: RenderContext<'_>) -> u16 {
        self.view.input_desired_height(width)
    }

    fn render(&self, frame: &mut Frame<'_>, area: Rect, context: RenderContext<'_>) {
        chat_input::draw_chat_input(
            frame,
            area,
            self.view.input(),
            self.view.input_cursor_width(),
            self.view.input_cursor_line(),
            self.view.input_prompt(),
            self.cursor,
            context,
        );
    }
}

pub(crate) fn draw_completion_layer(
    frame: &mut Frame<'_>,
    area: Rect,
    view: &ChatComposerView<'_>,
    hovered: Option<ChatComposerPointerTarget>,
    pressed: Option<ChatComposerPointerTarget>,
    context: RenderContext<'_>,
) {
    chat_input::draw_completion(
        frame,
        area,
        view.input_completion(),
        match hovered {
            Some(ChatComposerPointerTarget::CompletionItem(index)) => Some(index),
            None => None,
        },
        match pressed {
            Some(ChatComposerPointerTarget::CompletionItem(index)) => Some(index),
            None => None,
        },
        context,
    );
}

pub(crate) fn pointer_target_at(
    overlay_area: Rect,
    view: &ChatComposerView<'_>,
    completion_visible: bool,
    column: u16,
    row: u16,
) -> Option<ChatComposerPointerTarget> {
    completion_visible
        .then(|| {
            chat_input::completion_index_at(overlay_area, view.input_completion(), column, row)
        })
        .flatten()
        .map(ChatComposerPointerTarget::CompletionItem)
}
