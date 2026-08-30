//! One-row presentation of the canonical Thread Goal.

use crate::ui::muted;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Paragraph;
use zeta_protocol::ThreadGoal;

pub(crate) fn desired_height(goal: Option<&ThreadGoal>) -> u16 {
    u16::from(goal.is_some())
}

pub(crate) fn draw(frame: &mut Frame<'_>, area: Rect, goal: Option<&ThreadGoal>) {
    let Some(goal) = goal else {
        return;
    };
    frame.render_widget(
        Paragraph::new(format!("Goal: {}", goal.objective)).style(Style::default().fg(muted())),
        area,
    );
}
