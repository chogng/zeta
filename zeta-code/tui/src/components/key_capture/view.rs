use super::KeyCapture;
use crate::ui::highlight;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;

pub(crate) fn draw(frame: &mut Frame<'_>, area: Rect, capture: &KeyCapture) {
    frame.render_widget(
        Paragraph::new(
            capture
                .lines()
                .iter()
                .map(|line| Line::from(line.as_str()))
                .collect::<Vec<_>>(),
        )
        .block(
            Block::default()
                .title(capture.title())
                .borders(Borders::TOP)
                .border_style(Style::default().fg(highlight())),
        ),
        area,
    );
}
