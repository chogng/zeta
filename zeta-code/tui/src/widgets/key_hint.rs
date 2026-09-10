use crate::config::KeyHintStyle;
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct KeyHints {
    entries: Vec<KeyHint>,
    text: String,
    separator: &'static str,
}

impl Default for KeyHints {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            text: String::new(),
            separator: SEPARATOR,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum KeyHint {
    Action { keys: String, suffix: String },
    Note(String),
}

impl KeyHints {
    pub(crate) fn with_binding(self, shortcut: crate::keymap::bindings::Keybinding) -> Self {
        self.with_action(shortcut.keys(), shortcut.action())
    }

    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn compact() -> Self {
        Self {
            separator: " · ",
            ..Self::default()
        }
    }

    pub(crate) fn with_action(self, keys: impl Into<String>, action: impl Into<String>) -> Self {
        self.with_action_suffix(keys, format!(" to {}", action.into()))
    }

    pub(crate) fn with_compact_action(
        self,
        keys: impl Into<String>,
        action: impl Into<String>,
    ) -> Self {
        self.with_action_suffix(keys, format!(" {}", action.into()))
    }

    fn with_action_suffix(mut self, keys: impl Into<String>, suffix: String) -> Self {
        self.push(KeyHint::Action {
            keys: keys.into(),
            suffix,
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
            self.text.push_str(self.separator);
        }
        match &entry {
            KeyHint::Action { keys, suffix } => {
                self.text.push_str(keys);
                self.text.push_str(suffix);
            }
            KeyHint::Note(note) => self.text.push_str(note),
        }
        self.entries.push(entry);
    }
}

pub(crate) fn draw(
    frame: &mut Frame<'_>,
    area: Rect,
    hints: &KeyHints,
    style: KeyHintStyle,
    context: RenderContext<'_>,
) {
    let content = horizontal_margin(area, 2);
    draw_content(frame, content, hints, style, context);
}

pub(crate) fn draw_content(
    frame: &mut Frame<'_>,
    content: Rect,
    hints: &KeyHints,
    style: KeyHintStyle,
    context: RenderContext<'_>,
) {
    frame.render_widget(
        Paragraph::new(line(hints, content.width.into(), style, context)),
        content,
    );
}

fn line<'a>(
    hints: &'a KeyHints,
    width: usize,
    style: KeyHintStyle,
    context: RenderContext<'_>,
) -> Line<'a> {
    let (entries, shortened) = visible_entries(hints, width);
    let separator = if shortened { " · " } else { hints.separator };
    let muted = Style::default().fg(context.muted());
    if style == KeyHintStyle::Muted {
        return Line::from(Span::styled(
            visible_text(&entries, separator),
            muted.add_modifier(Modifier::ITALIC),
        ));
    }

    let mut spans = Vec::new();
    for (index, entry) in entries.into_iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(separator, muted));
        }
        match entry {
            KeyHint::Action { keys, suffix } => {
                spans.push(Span::styled(
                    keys.as_str(),
                    Style::default()
                        .fg(context.foreground())
                        .add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::styled(suffix.as_str(), muted));
            }
            KeyHint::Note(note) => spans.push(Span::styled(note.as_str(), muted)),
        }
    }
    Line::from(spans)
}

fn visible_entries(hints: &KeyHints, width: usize) -> (Vec<&KeyHint>, bool) {
    if hints.text.width() <= width {
        return (hints.entries.iter().collect(), false);
    }
    let mut entries = hints.entries.iter().collect::<Vec<_>>();
    while entries.len() > 1 {
        let index = entries
            .iter()
            .position(
                |entry| matches!(entry, KeyHint::Action { keys, .. } if keys.starts_with("↑↓")),
            )
            .or_else(|| {
                entries.iter().rposition(|entry| {
                    !matches!(
                        entry,
                        KeyHint::Action { keys, suffix }
                            if *keys == crate::keymap::bindings::CLOSE.keys()
                                && suffix.starts_with(" to ")
                    )
                })
            });
        let Some(index) = index else {
            break;
        };
        entries.remove(index);
        let text = visible_text(&entries, " · ");
        if text.width() <= width {
            return (entries, true);
        }
    }
    (entries, true)
}

fn visible_text(entries: &[&KeyHint], separator: &str) -> String {
    entries
        .iter()
        .map(|entry| match entry {
            KeyHint::Action { keys, suffix } => format!("{keys}{suffix}"),
            KeyHint::Note(note) => (*note).clone(),
        })
        .collect::<Vec<_>>()
        .join(separator)
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
