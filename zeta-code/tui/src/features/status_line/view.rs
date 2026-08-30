use super::StatusLineModel;
use super::StatusLineRuntime;
use super::model::ApprovalModeStatus;
use super::model::approval_mode_display;
use super::model::approval_mode_text;
use crate::ui::accent;
use crate::ui::chat_input_chrome;
use crate::ui::danger;
use crate::ui::warning;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use zeta_protocol::ApprovalMode;

pub(crate) fn draw(
    frame: &mut Frame<'_>,
    area: Rect,
    status_line: &StatusLineModel,
    approval: ApprovalModeStatus,
    runtime: StatusLineRuntime,
) {
    if area.is_empty() {
        return;
    }

    let runtime_text = runtime.text();
    let configured = status_line.text_for_width(usize::from(area.width), approval);
    let lines = if runtime_text.is_empty() {
        vec![styled_line(configured, approval)]
    } else if configured.is_empty() || area.height == 1 {
        vec![Line::styled(
            runtime_text,
            Style::default().fg(chat_input_chrome()),
        )]
    } else {
        vec![
            Line::styled(runtime_text, Style::default().fg(chat_input_chrome())),
            styled_line(configured, approval),
        ]
    };
    frame.render_widget(Paragraph::new(lines), area);
}

pub(crate) fn desired_rows(runtime: StatusLineRuntime, max_rows: u16) -> u16 {
    let desired = if runtime.text().is_empty() { 1 } else { 2 };
    desired.min(max_rows.max(1))
}

fn styled_line(text: String, approval: ApprovalModeStatus) -> Line<'static> {
    let permission_prefix = approval_mode_text(approval);
    let Some(remainder) = text.strip_prefix(&permission_prefix) else {
        return Line::styled(text, Style::default().fg(chat_input_chrome()));
    };

    let next = approval_mode_display(approval.next);
    if let Some(current_mode) = approval.current.filter(|current| *current != approval.next) {
        let current = approval_mode_display(current_mode);
        return Line::from(vec![
            Span::styled(current.icon, Style::default().fg(mode_color(current_mode))),
            Span::styled(
                format!(" current: {} · ", current.label),
                Style::default().fg(chat_input_chrome()),
            ),
            Span::styled(next.icon, Style::default().fg(mode_color(approval.next))),
            Span::styled(
                format!(" next: {}{remainder}", next.label),
                Style::default().fg(chat_input_chrome()),
            ),
        ]);
    }

    Line::from(vec![
        Span::styled(next.icon, Style::default().fg(mode_color(approval.next))),
        Span::styled(
            format!(" {}{remainder}", next.label),
            Style::default().fg(chat_input_chrome()),
        ),
    ])
}

fn mode_color(approval_mode: ApprovalMode) -> ratatui::style::Color {
    match approval_mode {
        ApprovalMode::AskPermissions => warning(),
        ApprovalMode::AutoReview => accent(),
        ApprovalMode::BypassPermissions => danger(),
    }
}
