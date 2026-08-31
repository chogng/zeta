use super::StatusLineModel;
use super::StatusLineRuntime;
use super::model::ApprovalModeStatus;
use super::model::approval_mode_display;
use super::model::approval_mode_text;
use crate::render::RenderContext;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use unicode_width::UnicodeWidthStr;
use zeta_protocol::ApprovalMode;

const POLICY_HINT: &str = " (shift+tab to cycle)";

pub(crate) fn draw(
    frame: &mut Frame<'_>,
    area: Rect,
    status_line: &StatusLineModel,
    approval: ApprovalModeStatus,
    runtime: StatusLineRuntime,
    context: RenderContext<'_>,
) {
    if area.is_empty() {
        return;
    }

    let top = status_line.top_text_for_width(usize::from(area.width), runtime);
    let hint_width = POLICY_HINT.width();
    let policy_width = usize::from(area.width);
    let full_policy = status_line.policy_text_for_width(usize::MAX, approval);
    let show_hint =
        !full_policy.is_empty() && full_policy.width().saturating_add(hint_width) <= policy_width;
    let policy = status_line.policy_text_for_width(
        policy_width.saturating_sub(if show_hint { hint_width } else { 0 }),
        approval,
    );
    let lines = if area.height == 1 {
        if policy.is_empty() {
            vec![top_line(top, context)]
        } else {
            vec![styled_policy_line(policy, approval, show_hint, context)]
        }
    } else if top.is_empty() {
        vec![styled_policy_line(policy, approval, show_hint, context)]
    } else if policy.is_empty() {
        vec![top_line(top, context)]
    } else {
        vec![
            top_line(top, context),
            styled_policy_line(policy, approval, show_hint, context),
        ]
    };
    frame.render_widget(Paragraph::new(lines), area);
}

pub(crate) fn desired_rows(
    status_line: &StatusLineModel,
    approval: ApprovalModeStatus,
    runtime: StatusLineRuntime,
    max_rows: u16,
) -> u16 {
    let top = status_line.top_text_for_width(usize::MAX, runtime);
    let policy = status_line.policy_text_for_width(usize::MAX, approval);
    let desired = if top.is_empty() || policy.is_empty() {
        1
    } else {
        2
    };
    desired.min(max_rows.max(1))
}

fn top_line(text: String, context: RenderContext<'_>) -> Line<'static> {
    Line::styled(text, Style::default().fg(context.chat_input_chrome()))
}

fn styled_policy_line(
    policy: String,
    approval: ApprovalModeStatus,
    show_hint: bool,
    context: RenderContext<'_>,
) -> Line<'static> {
    let permission_prefix = approval_mode_text(approval);
    if policy != permission_prefix {
        return Line::styled(policy, Style::default().fg(context.chat_input_chrome()));
    }
    let hint = show_hint.then_some(Span::styled(
        POLICY_HINT,
        Style::default().fg(context.muted()),
    ));
    let next = approval_mode_display(approval.next);
    if let Some(current_mode) = approval.current.filter(|current| *current != approval.next) {
        let current = approval_mode_display(current_mode);
        let mut spans = vec![
            Span::styled(
                current.icon,
                Style::default().fg(mode_color(current_mode, context)),
            ),
            Span::styled(
                format!(" current: {} · ", current.label),
                Style::default().fg(context.chat_input_chrome()),
            ),
            Span::styled(
                next.icon,
                Style::default().fg(mode_color(approval.next, context)),
            ),
            Span::styled(
                format!(" next: {}", next.label),
                Style::default().fg(context.chat_input_chrome()),
            ),
        ];
        spans.extend(hint);
        return Line::from(spans);
    }

    let mut spans = vec![
        Span::styled(
            next.icon,
            Style::default().fg(mode_color(approval.next, context)),
        ),
        Span::styled(
            format!(" {}", next.label),
            Style::default().fg(context.chat_input_chrome()),
        ),
    ];
    spans.extend(hint);
    Line::from(spans)
}

fn mode_color(approval_mode: ApprovalMode, context: RenderContext<'_>) -> ratatui::style::Color {
    match approval_mode {
        ApprovalMode::AskPermissions => context.warning(),
        ApprovalMode::AutoReview => context.accent(),
        ApprovalMode::BypassPermissions => context.danger(),
    }
}
