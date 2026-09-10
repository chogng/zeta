use super::ChatComposerView;
use crate::render::RenderContext;
use crate::render::Renderable;
use crate::thread::composer as chat_input;
use crate::thread::composer::ChatInputCursor;
use crate::thread::composer::ChatInputFocus;
use ratatui::Frame;
use ratatui::layout::Rect;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ChatComposerPointerTarget {
    Input,
    CompletionItem(usize),
}

pub(crate) struct ChatComposerSurface<'a, 'view> {
    pub(crate) view: &'view ChatComposerView<'a>,
    pub(crate) cursor: ChatInputCursor,
    pub(crate) focus: ChatInputFocus,
    pub(crate) chrome: chat_input::ChatInputChrome,
}

impl Renderable for ChatComposerSurface<'_, '_> {
    fn desired_height(&self, width: u16, _context: RenderContext<'_>) -> u16 {
        self.view
            .input_desired_height(width.saturating_sub(self.chrome.inset(width)))
    }

    fn render(&self, frame: &mut Frame<'_>, area: Rect, context: RenderContext<'_>) {
        chat_input::draw_chat_input(
            frame,
            area,
            self.view.input(),
            self.view.input_cursor_width(),
            self.view.input_cursor_line(),
            self.view.input_prompt(),
            if self.view.searching_history() {
                ChatInputCursor::Hidden
            } else {
                self.cursor
            },
            self.focus,
            self.chrome,
            context,
        );
        if let Some(status) = self.view.history_status() {
            let content = match self.chrome {
                chat_input::ChatInputChrome::Rules => chat_input::content_area(area),
                chat_input::ChatInputChrome::Box => {
                    crate::render::horizontal_margin(self.chrome.border_area(area), 2)
                }
            };
            frame.render_widget(
                ratatui::widgets::Paragraph::new(status)
                    .style(ratatui::style::Style::default().fg(context.foreground())),
                Rect::new(content.x, area.y, content.width, 1),
            );
        }
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
            Some(ChatComposerPointerTarget::Input) => None,
            Some(ChatComposerPointerTarget::CompletionItem(index)) => Some(index),
            None => None,
        },
        match pressed {
            Some(ChatComposerPointerTarget::Input) => None,
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
