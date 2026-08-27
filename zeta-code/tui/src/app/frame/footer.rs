//! Status-specific interaction hints.

use crate::app::App;
use crate::features::status_line;
use crate::ui::warning;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Paragraph;

pub(super) fn draw(frame: &mut Frame<'_>, area: Rect, app: &App) {
    if let Some(prefix) = app.pending_key_chord_label() {
        frame.render_widget(
            Paragraph::new(format!("{prefix} … waiting for next key · esc cancel"))
                .style(Style::default().fg(warning())),
            area,
        );
        return;
    }
    status_line::draw(frame, area, app.status_line(), app.approval_mode());
}
