//! Status-specific interaction hints.

use crate::app::App;
use crate::app::Status;
use crate::ui::{danger, muted, warning};
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
    let mode = match app.approval_mode() {
        zeta_protocol::ApprovalMode::AskPermissions => "ask permissions on",
        zeta_protocol::ApprovalMode::AutoReview => "auto review on",
        zeta_protocol::ApprovalMode::BypassPermissions => "bypass permissions on",
    };
    let (text, style) = match app.status() {
        Status::Ready => (mode.into(), Style::default().fg(muted())),
        Status::Working => (format!("{mode} · working…"), Style::default().fg(warning())),
        Status::WaitingForApproval => (
            format!("{mode} · approval required"),
            Style::default().fg(warning()),
        ),
        Status::WaitingForUserInput => (
            format!("{mode} · input required"),
            Style::default().fg(warning()),
        ),
        Status::WaitingForCapability => (
            format!("{mode} · capability required"),
            Style::default().fg(warning()),
        ),
        Status::Cancelling => (
            format!("{mode} · interrupting…"),
            Style::default().fg(warning()),
        ),
        Status::Error => (
            format!("{mode} · ready to retry"),
            Style::default().fg(danger()),
        ),
    };
    frame.render_widget(Paragraph::new(text).style(style), area);
}
