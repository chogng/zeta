use super::QueryCustomAnswer;
use super::QueryView;
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

const MAX_CHOICE_ROWS: usize = 8;

pub(crate) fn desired_height(view: QueryView<'_>) -> u16 {
    let choice_count = view.question.choices.len()
        + usize::from(view.question.custom_answer == QueryCustomAnswer::Allowed);
    let content_rows = 2usize
        .saturating_add(choice_count.min(MAX_CHOICE_ROWS))
        .saturating_add(usize::from(view.submitting || view.error.is_some()));
    u16::try_from(content_rows.saturating_add(2)).unwrap_or(u16::MAX)
}

pub(crate) fn draw(frame: &mut Frame<'_>, area: Rect, view: QueryView<'_>) {
    let mut lines = vec![Line::styled(
        &view.question.prompt,
        Style::default().add_modifier(Modifier::BOLD),
    )];
    lines.extend(
        view.question
            .choices
            .iter()
            .enumerate()
            .take(MAX_CHOICE_ROWS)
            .map(|(index, choice)| {
                choice_line(&choice.label, &choice.description, index == view.selected)
            }),
    );
    if view.question.custom_answer == QueryCustomAnswer::Allowed
        && view.question.choices.len() < MAX_CHOICE_ROWS
    {
        lines.push(choice_line(
            "自己输入",
            "在下方输入框中回答",
            view.selected == view.question.choices.len(),
        ));
    }
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
                .title(format!(
                    "{}  ({}/{})",
                    view.question.header,
                    view.current + 1,
                    view.total
                ))
                .borders(Borders::ALL)
                .style(Style::default().bg(background())),
        ),
        area,
    );
}

pub(crate) fn choice_index_at(
    area: Rect,
    view: QueryView<'_>,
    column: u16,
    row: u16,
) -> Option<usize> {
    if column <= area.x || column >= area.right().saturating_sub(1) {
        return None;
    }
    let first_choice_row = area.y.saturating_add(2);
    let visible_choices = view.question.choices.len().min(MAX_CHOICE_ROWS)
        + usize::from(
            view.question.custom_answer == QueryCustomAnswer::Allowed
                && view.question.choices.len() < MAX_CHOICE_ROWS,
        );
    let index = usize::from(row.saturating_sub(first_choice_row));
    (row >= first_choice_row && index < visible_choices).then_some(index)
}

fn choice_line<'a>(label: &'a str, description: &'a str, selected: bool) -> Line<'a> {
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
        Span::styled(format!("  {description}"), Style::default().fg(muted())),
    ])
}
