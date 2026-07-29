//! Product header presentation.

use crate::app::Status;
use crate::ui::ACCENT;
use crate::ui::DANGER;
use crate::ui::MUTED;
use crate::ui::SUCCESS;
use crate::ui::WARNING;
use ratatui::Frame;
use ratatui::layout::Alignment;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use unicode_width::UnicodeWidthStr;

pub(super) fn draw(frame: &mut Frame<'_>, area: Rect, status: &Status) {
    let status_text = status_label(status);
    let status_width = status_text.width().min(u16::MAX as usize) as u16;
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(12),
            Constraint::Length(status_width.saturating_add(3)),
        ])
        .split(area);
    let divider = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(MUTED));
    frame.render_widget(divider, area);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                "  Zeta",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  workspace assistant", Style::default().fg(MUTED)),
        ])),
        columns[0],
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("● ", Style::default().fg(status_color(status))),
            Span::styled(status_text, Style::default().fg(MUTED)),
        ]))
        .alignment(Alignment::Right),
        columns[1],
    );
}

fn status_label(status: &Status) -> &'static str {
    match status {
        Status::Ready => "ready",
        Status::Working => "working",
        Status::WaitingForApproval => "approval",
        Status::WaitingForUserInput => "waiting",
        Status::WaitingForCapability => "capability",
        Status::Cancelling => "stopping",
        Status::Error => "attention",
    }
}

fn status_color(status: &Status) -> Color {
    match status {
        Status::Ready => SUCCESS,
        Status::Working
        | Status::WaitingForApproval
        | Status::WaitingForUserInput
        | Status::WaitingForCapability
        | Status::Cancelling => WARNING,
        Status::Error => DANGER,
    }
}
