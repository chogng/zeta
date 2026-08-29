use super::PlanProgressView;
use crate::ui::background;
use crate::ui::highlight;
use crate::ui::muted;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use zeta_protocol::PlanStepStatus;

const MAX_VISIBLE_STEPS: usize = 6;

pub(crate) fn desired_height(view: PlanProgressView<'_>) -> u16 {
    let body_height = if view.expanded {
        view.plan.steps.len().min(MAX_VISIBLE_STEPS)
    } else {
        1
    };
    u16::try_from(body_height.saturating_add(2)).unwrap_or(u16::MAX)
}

pub(crate) fn draw(frame: &mut Frame<'_>, area: Rect, view: PlanProgressView<'_>) {
    let completed = view.completed_steps();
    let total = view.plan.steps.len();
    let title = format!(
        "{} Plan  {completed}/{total}",
        if view.expanded { "▾" } else { "▸" }
    );
    let lines = if view.expanded {
        view.plan
            .steps
            .iter()
            .take(MAX_VISIBLE_STEPS)
            .map(|step| {
                let (marker, style) = match step.status {
                    PlanStepStatus::Completed => ("✓", Style::default().fg(muted())),
                    PlanStepStatus::InProgress => (
                        "›",
                        Style::default()
                            .fg(highlight())
                            .add_modifier(Modifier::BOLD),
                    ),
                    PlanStepStatus::Pending => ("·", Style::default()),
                };
                Line::styled(format!("{marker} {}", step.step), style)
            })
            .collect::<Vec<_>>()
    } else {
        let current = view
            .plan
            .steps
            .iter()
            .find(|step| step.status == PlanStepStatus::InProgress)
            .or_else(|| {
                view.plan
                    .steps
                    .iter()
                    .find(|step| step.status == PlanStepStatus::Pending)
            })
            .or_else(|| view.plan.steps.last());
        vec![Line::from(
            current
                .map(|step| step.step.as_str())
                .unwrap_or("Plan has no steps"),
        )]
    };
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(highlight()))
                .style(Style::default().bg(background())),
        ),
        area,
    );
}
