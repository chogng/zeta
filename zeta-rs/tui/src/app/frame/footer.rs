//! Status-specific interaction hints.

use crate::app::Status;
use crate::ui::DANGER;
use crate::ui::MUTED;
use crate::ui::WARNING;
use ratatui::Frame;
use ratatui::layout::Alignment;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Paragraph;

pub(super) fn draw(frame: &mut Frame<'_>, area: Rect, status: &Status) {
    let (text, style) = match status {
        Status::Ready => (
            "policy  (shift + tab to cycle)  ·  enter send  ·  ctrl-v image  ·  esc quit",
            Style::default().fg(MUTED),
        ),
        Status::Working => (
            "working…  ·  ctrl-c interrupt",
            Style::default().fg(WARNING),
        ),
        Status::WaitingForApproval => (
            "approval required  ·  ctrl-c interrupt",
            Style::default().fg(WARNING),
        ),
        Status::WaitingForUserInput => (
            "input required  ·  ctrl-c interrupt",
            Style::default().fg(WARNING),
        ),
        Status::WaitingForCapability => (
            "capability required  ·  ctrl-c interrupt",
            Style::default().fg(WARNING),
        ),
        Status::Cancelling => ("interrupting…", Style::default().fg(WARNING)),
        Status::Error => ("ready to retry  ·  esc quit", Style::default().fg(DANGER)),
    };
    frame.render_widget(
        Paragraph::new(text)
            .style(style)
            .alignment(Alignment::Center),
        area,
    );
}
