use super::ApprovalDecision;
use super::ApprovalView;
use crate::ui::background;
use crate::ui::highlight;
use crate::ui::muted;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;

const MAX_DETAIL_ROWS: usize = 5;

pub(crate) fn desired_height(view: ApprovalView<'_>) -> u16 {
    let content_rows = 4usize
        .saturating_add(view.details.len().min(MAX_DETAIL_ROWS))
        .saturating_add(usize::from(view.error.is_some()));
    u16::try_from(content_rows.saturating_add(2)).unwrap_or(u16::MAX)
}

pub(crate) fn draw(frame: &mut Frame<'_>, area: Rect, view: ApprovalView<'_>) {
    let mut lines = vec![Line::styled(
        view.reason,
        Style::default().add_modifier(Modifier::BOLD),
    )];
    lines.extend(
        view.details
            .iter()
            .take(MAX_DETAIL_ROWS)
            .map(|detail| Line::styled(detail, Style::default().fg(muted()))),
    );
    lines.push(choice_line(
        "Approve once",
        view.selected == ApprovalDecision::ApproveOnce,
    ));
    lines.push(choice_line(
        "Decline",
        view.selected == ApprovalDecision::Decline,
    ));
    if view.submitting {
        lines.push(Line::styled("Submitting…", Style::default().fg(muted())));
    } else if let Some(error) = view.error {
        lines.push(Line::styled(
            error,
            Style::default().fg(ratatui::style::Color::Red),
        ));
    }
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title(view.title)
                .borders(Borders::ALL)
                .style(Style::default().bg(background())),
        ),
        area,
    );
}

pub(crate) fn choice_index_at(
    area: Rect,
    view: ApprovalView<'_>,
    column: u16,
    row: u16,
) -> Option<usize> {
    if column <= area.x || column >= area.right().saturating_sub(1) {
        return None;
    }
    let first_choice_row = area
        .y
        .saturating_add(1)
        .saturating_add(1)
        .saturating_add(view.details.len().min(MAX_DETAIL_ROWS) as u16);
    let index = usize::from(row.saturating_sub(first_choice_row));
    (row >= first_choice_row && index < 2).then_some(index)
}

fn choice_line(label: &str, selected: bool) -> Line<'_> {
    let marker = if selected { "› " } else { "  " };
    let style = if selected {
        Style::default()
            .fg(highlight())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    Line::from(vec![
        Span::styled(marker, style),
        Span::styled(label, style),
    ])
}
