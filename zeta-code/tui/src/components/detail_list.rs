#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DetailListRow {
    label: String,
    value: String,
}

impl DetailListRow {
    pub(crate) fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }

    pub(crate) fn label(&self) -> &str {
        &self.label
    }

    pub(crate) fn value(&self) -> &str {
        &self.value
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DetailList {
    title: String,
    rows: Vec<DetailListRow>,
}

impl DetailList {
    pub(crate) fn new(title: impl Into<String>, rows: Vec<DetailListRow>) -> Self {
        Self {
            title: title.into(),
            rows,
        }
    }

    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    pub(crate) fn rows(&self) -> &[DetailListRow] {
        &self.rows
    }

    pub(crate) fn desired_height(&self) -> u16 {
        let content_rows = self
            .rows
            .iter()
            .map(|row| row.value.lines().count().max(1))
            .sum::<usize>();
        u16::try_from(content_rows.saturating_add(3)).unwrap_or(u16::MAX)
    }
}

use crate::ui::highlight;
use crate::ui::horizontal_margin;
use crate::ui::muted;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;

pub(crate) fn draw_scrolled(frame: &mut Frame<'_>, area: Rect, detail: &DetailList, scroll: u16) {
    let lines = detail
        .rows()
        .iter()
        .map(|row| {
            Line::from(vec![
                Span::styled(
                    format!("{}: ", row.label()),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::styled(row.value(), Style::default().fg(muted())),
            ])
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(lines).scroll((scroll, 0)).block(
            Block::default()
                .title(detail.title())
                .borders(Borders::TOP)
                .border_style(Style::default().fg(highlight())),
        ),
        horizontal_margin(area, 2),
    );
}

#[cfg(test)]
#[path = "detail_list_tests.rs"]
mod tests;
