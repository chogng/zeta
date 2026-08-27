use super::StatusLineModel;
use crate::ui::composer_chrome;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Paragraph;
use zeta_protocol::ApprovalMode;

pub(crate) fn draw(
    frame: &mut Frame<'_>,
    area: Rect,
    status_line: &StatusLineModel,
    approval_mode: ApprovalMode,
) {
    if area.is_empty() {
        return;
    }

    frame.render_widget(
        Paragraph::new(status_line.text_for_width(usize::from(area.width), approval_mode))
            .style(Style::default().fg(composer_chrome())),
        area,
    );
}
