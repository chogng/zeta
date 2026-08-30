use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use std::fmt;

pub(crate) const SEARCH_BOX_HEIGHT: u16 = 3;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SearchBoxModel {
    placeholder: String,
    initially_active: bool,
    masked: bool,
}

impl SearchBoxModel {
    pub(crate) fn new(placeholder: impl Into<String>) -> Self {
        Self {
            placeholder: placeholder.into(),
            initially_active: false,
            masked: false,
        }
    }

    pub(crate) fn initially_active(mut self) -> Self {
        self.initially_active = true;
        self
    }

    pub(crate) fn masked(mut self) -> Self {
        self.masked = true;
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SearchBoxInputOutcome {
    Ignored,
    QueryChanged,
}

#[derive(Clone, Eq, PartialEq)]
pub(crate) struct SearchBoxState {
    model: SearchBoxModel,
    query: String,
    input_active: bool,
}

impl SearchBoxState {
    pub(crate) fn new(model: SearchBoxModel) -> Self {
        Self {
            input_active: model.initially_active,
            model,
            query: String::new(),
        }
    }

    pub(crate) fn replace_model(&mut self, model: SearchBoxModel) {
        self.model = model;
    }

    pub(crate) fn placeholder(&self) -> &str {
        &self.model.placeholder
    }

    pub(crate) fn query(&self) -> &str {
        &self.query
    }

    pub(crate) fn input_active(&self) -> bool {
        self.input_active
    }

    pub(crate) fn set_input_active(&mut self, input_active: bool) {
        self.input_active = input_active;
    }

    pub(crate) fn masked(&self) -> bool {
        self.model.masked
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> SearchBoxInputOutcome {
        if !self.input_active {
            return SearchBoxInputOutcome::Ignored;
        }

        match key.code {
            KeyCode::Backspace => {
                self.query.pop();
                SearchBoxInputOutcome::QueryChanged
            }
            KeyCode::Char(character) if !character.is_ascii_control() => {
                self.query.push(character);
                SearchBoxInputOutcome::QueryChanged
            }
            _ => SearchBoxInputOutcome::Ignored,
        }
    }

    pub(crate) fn handle_paste(&mut self, pasted: String) -> SearchBoxInputOutcome {
        if !self.input_active {
            return SearchBoxInputOutcome::Ignored;
        }
        let normalized = pasted.split_whitespace().collect::<Vec<_>>().join(" ");
        self.query.push_str(&normalized);
        SearchBoxInputOutcome::QueryChanged
    }
}

impl fmt::Debug for SearchBoxState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SearchBoxState")
            .field("model", &self.model)
            .field(
                "query",
                &if self.masked() {
                    "[REDACTED]"
                } else {
                    self.query.as_str()
                },
            )
            .field("input_active", &self.input_active)
            .finish()
    }
}

use crate::render::RenderContext;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Padding;
use ratatui::widgets::Paragraph;
use unicode_width::UnicodeWidthStr;

const SEARCH_BOX_LEFT_PADDING: u16 = 1;

pub(crate) fn draw(
    frame: &mut Frame<'_>,
    area: Rect,
    search: &SearchBoxState,
    active_color: Color,
    context: RenderContext<'_>,
) {
    let rendered_query = search
        .masked()
        .then(|| "•".repeat(search.query().chars().count()));
    let text = if search.query().is_empty() {
        Span::styled(search.placeholder(), Style::default().fg(context.muted()))
    } else {
        Span::raw(rendered_query.as_deref().unwrap_or(search.query()))
    };
    let border_color = if search.input_active() {
        active_color
    } else {
        context.muted()
    };
    frame.render_widget(
        Paragraph::new(Line::from(text)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .padding(Padding::left(SEARCH_BOX_LEFT_PADDING)),
        ),
        area,
    );
    if search.input_active() && area.width > 2 && area.height > 2 {
        let cursor_width = rendered_query
            .as_deref()
            .unwrap_or(search.query())
            .width()
            .min(usize::from(
                area.width
                    .saturating_sub(2)
                    .saturating_sub(SEARCH_BOX_LEFT_PADDING)
                    .saturating_sub(1),
            )) as u16;
        frame.set_cursor_position((
            area.x + 1 + SEARCH_BOX_LEFT_PADDING + cursor_width,
            area.y + 1,
        ));
    }
}

#[cfg(test)]
#[path = "search_box_state_tests.rs"]
mod state_tests;

#[cfg(test)]
#[path = "search_box_view_tests.rs"]
mod view_tests;
