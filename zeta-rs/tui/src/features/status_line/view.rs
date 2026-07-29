use super::StatusLineModel;
use crate::ui::COMPOSER_CHROME;
use crate::ui::horizontal_margin;
use ratatui::Frame;
use ratatui::layout::Alignment;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Paragraph;

pub(crate) fn draw(frame: &mut Frame<'_>, area: Rect, status_line: &StatusLineModel) {
    let area = horizontal_margin(area, 2);
    let text = status_line.text_for_width(area.width as usize);
    frame.render_widget(
        Paragraph::new(text)
            .style(Style::default().fg(COMPOSER_CHROME))
            .alignment(Alignment::Right),
        area,
    );
}
