use crate::render::RenderContext;
use crate::render::horizontal_margin;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use unicode_width::UnicodeWidthStr;

const SEPARATOR: &str = "  ·  ";

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct KeyHints {
    entries: Vec<KeyHint>,
    text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum KeyHint {
    Action { keys: String, label: String },
    Note(String),
}

impl KeyHints {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn with(mut self, keys: impl Into<String>, label: impl Into<String>) -> Self {
        self.push(KeyHint::Action {
            keys: keys.into(),
            label: label.into(),
        });
        self
    }

    pub(crate) fn with_note(mut self, note: impl Into<String>) -> Self {
        self.push(KeyHint::Note(note.into()));
        self
    }

    pub(crate) fn extend(mut self, other: Self) -> Self {
        for entry in other.entries {
            self.push(entry);
        }
        self
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    fn push(&mut self, entry: KeyHint) {
        if !self.text.is_empty() {
            self.text.push_str(SEPARATOR);
        }
        match &entry {
            KeyHint::Action { keys, label } => {
                self.text.push_str(keys);
                self.text.push(' ');
                self.text.push_str(label);
            }
            KeyHint::Note(note) => self.text.push_str(note),
        }
        self.entries.push(entry);
    }
}

pub(crate) fn draw(frame: &mut Frame<'_>, area: Rect, hints: &str, context: RenderContext<'_>) {
    let content = horizontal_margin(area, 2);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            hints,
            Style::default()
                .fg(context.muted())
                .add_modifier(Modifier::ITALIC),
        ))),
        content,
    );
}

pub(crate) fn draw_right(
    frame: &mut Frame<'_>,
    area: Rect,
    hints: &str,
    context: RenderContext<'_>,
) {
    let content = horizontal_margin(area, 2);
    let width = hints.width().min(usize::from(content.width)) as u16;
    let hint_area = Rect {
        x: content.right().saturating_sub(width),
        width,
        ..content
    };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            hints,
            Style::default()
                .fg(context.muted())
                .add_modifier(Modifier::ITALIC),
        ))),
        hint_area,
    );
}

#[cfg(test)]
#[path = "key_hint_tests.rs"]
mod tests;
