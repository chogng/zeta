//! Status-specific interaction hints.

use crate::app::Status;
use crate::ui::{danger, muted, warning};
use ratatui::Frame;
use ratatui::layout::Alignment;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Paragraph;

pub(super) fn draw(frame: &mut Frame<'_>, area: Rect, status: &Status) {
    let (text, style) = match status {
        Status::Ready => (
            "policy  (shift + tab to cycle)  ·  enter send  ·  ctrl-v image  ·  ctrl-c quit",
            Style::default().fg(muted()),
        ),
        Status::Working => (
            "working…  ·  ctrl-c interrupt",
            Style::default().fg(warning()),
        ),
        Status::WaitingForApproval => (
            "approval required  ·  ctrl-c interrupt",
            Style::default().fg(warning()),
        ),
        Status::WaitingForUserInput => (
            "input required  ·  ctrl-c interrupt",
            Style::default().fg(warning()),
        ),
        Status::WaitingForCapability => (
            "capability required  ·  ctrl-c interrupt",
            Style::default().fg(warning()),
        ),
        Status::Cancelling => ("interrupting…", Style::default().fg(warning())),
        Status::Error => (
            "ready to retry  ·  ctrl-c quit",
            Style::default().fg(danger()),
        ),
    };
    frame.render_widget(
        Paragraph::new(text)
            .style(style)
            .alignment(Alignment::Center),
        area,
    );
}
