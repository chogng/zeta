//! Status-specific interaction hints.

use crate::app::App;
use crate::app::Status;
use crate::features::status_line;
use crate::ui::muted;
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
    if matches!(app.status(), Status::Working)
        && app.steers_active_turn()
        && !app.input().trim().is_empty()
    {
        frame.render_widget(
            Paragraph::new("enter steer · tab queue").style(Style::default().fg(muted())),
            area,
        );
        return;
    }
    status_line::draw(frame, area, app.status_line(), app.approval_mode_status());
}
