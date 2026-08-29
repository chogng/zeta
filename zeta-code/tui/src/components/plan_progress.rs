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
use zeta_protocol::PlanUpdate;

const MAX_VISIBLE_STEPS: usize = 6;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PlanProgress {
    plan: PlanUpdate,
    expanded: bool,
}

impl PlanProgress {
    pub(crate) fn new(plan: PlanUpdate) -> Self {
        Self {
            plan,
            expanded: false,
        }
    }

    pub(crate) fn replace(&mut self, plan: PlanUpdate) {
        self.plan = plan;
    }

    pub(crate) fn toggle_expanded(&mut self) {
        self.expanded = !self.expanded;
    }

    pub(crate) fn is_complete(&self) -> bool {
        !self.plan.steps.is_empty()
            && self
                .plan
                .steps
                .iter()
                .all(|step| step.status == PlanStepStatus::Completed)
    }

    pub(crate) fn view(&self) -> PlanProgressView<'_> {
        PlanProgressView {
            plan: &self.plan,
            expanded: self.expanded,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct PlanProgressView<'a> {
    pub(crate) plan: &'a PlanUpdate,
    pub(crate) expanded: bool,
}

impl PlanProgressView<'_> {
    pub(crate) fn completed_steps(self) -> usize {
        self.plan
            .steps
            .iter()
            .filter(|step| step.status == PlanStepStatus::Completed)
            .count()
    }
}

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

#[cfg(test)]
#[path = "plan_progress/state_tests.rs"]
mod tests;
