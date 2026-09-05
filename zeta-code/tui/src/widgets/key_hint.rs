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
    Action { keys: String, action: String },
    Note(String),
}

impl KeyHints {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn with_action(
        mut self,
        keys: impl Into<String>,
        action: impl Into<String>,
    ) -> Self {
        self.push(KeyHint::Action {
            keys: keys.into(),
            action: action.into(),
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
            KeyHint::Action { keys, action } => {
                self.text.push_str(keys);
                self.text.push_str(" to ");
                self.text.push_str(action);
            }
            KeyHint::Note(note) => self.text.push_str(note),
        }
        self.entries.push(entry);
    }
}

pub(crate) fn draw(frame: &mut Frame<'_>, area: Rect, hints: &str, context: RenderContext<'_>) {
    let content = horizontal_margin(area, 2);
    let hints = visible_hints(hints, usize::from(content.width));
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

fn visible_hints(hints: &str, width: usize) -> std::borrow::Cow<'_, str> {
    if hints.width() <= width {
        return std::borrow::Cow::Borrowed(hints);
    }
    let mut entries = hints.split('·').map(str::trim).collect::<Vec<_>>();
    while entries.len() > 1 {
        let index = entries
            .iter()
            .position(|entry| entry.starts_with("↑↓"))
            .or_else(|| {
                entries
                    .iter()
                    .rposition(|entry| !entry.starts_with("Esc to "))
            });
        let Some(index) = index else {
            break;
        };
        entries.remove(index);
        let text = entries.join(" · ");
        if text.width() <= width {
            return std::borrow::Cow::Owned(text);
        }
    }
    std::borrow::Cow::Owned(entries.join(" · "))
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
