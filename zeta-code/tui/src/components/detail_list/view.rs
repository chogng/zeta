use super::DetailList;
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

pub(crate) fn draw(frame: &mut Frame<'_>, area: Rect, detail: &DetailList) {
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
        Paragraph::new(lines).block(
            Block::default()
                .title(detail.title())
                .borders(Borders::TOP)
                .border_style(Style::default().fg(highlight())),
        ),
        horizontal_margin(area, 2),
    );
}
