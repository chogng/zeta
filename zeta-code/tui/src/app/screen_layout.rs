use ratatui::layout::Rect;

const MIN_TRANSCRIPT_ROWS: u16 = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SessionAreas {
    pub(crate) transcript: Rect,
    pub(crate) goal: Rect,
    pub(crate) plan: Rect,
    pub(crate) queue: Rect,
    pub(crate) request: Rect,
    pub(crate) composer: Rect,
    pub(crate) status: Rect,
    pub(crate) subagent_pane: Rect,
}

pub(crate) fn session_areas(
    area: Rect,
    goal_desired_rows: u16,
    plan_desired_rows: u16,
    queue_desired_rows: u16,
    request_desired_rows: u16,
    composer_desired_rows: u16,
    status_desired_rows: u16,
    subagent_pane_desired_rows: u16,
) -> SessionAreas {
    let subagent_pane_rows = subagent_pane_desired_rows.min(area.height);
    let available_above_subagent_pane = area.height.saturating_sub(subagent_pane_rows);
    let status_rows = status_desired_rows.min(available_above_subagent_pane);
    let available_above_status = available_above_subagent_pane.saturating_sub(status_rows);
    let transcript_rows = MIN_TRANSCRIPT_ROWS.min(available_above_status);
    let available_chrome = available_above_status.saturating_sub(transcript_rows);
    let composer_rows = composer_desired_rows.min(available_chrome);
    let request_rows = request_desired_rows.min(available_chrome.saturating_sub(composer_rows));
    let available_inline = available_chrome
        .saturating_sub(composer_rows)
        .saturating_sub(request_rows);
    let queue_rows = queue_desired_rows.min(available_inline);
    let plan_rows = plan_desired_rows.min(available_inline.saturating_sub(queue_rows));
    let goal_rows = goal_desired_rows.min(
        available_inline
            .saturating_sub(queue_rows)
            .saturating_sub(plan_rows),
    );
    let bottom = area.y.saturating_add(area.height);
    let subagent_pane_y = bottom.saturating_sub(subagent_pane_rows);
    let status_y = subagent_pane_y.saturating_sub(status_rows);
    let composer_y = status_y.saturating_sub(composer_rows);
    let request_y = composer_y.saturating_sub(request_rows);
    let queue_y = request_y.saturating_sub(queue_rows);
    let plan_y = queue_y.saturating_sub(plan_rows);
    let goal_y = plan_y.saturating_sub(goal_rows);

    SessionAreas {
        transcript: Rect {
            height: goal_y.saturating_sub(area.y),
            ..area
        },
        goal: Rect {
            y: goal_y,
            height: goal_rows,
            ..area
        },
        plan: Rect {
            y: plan_y,
            height: plan_rows,
            ..area
        },
        queue: Rect {
            y: queue_y,
            height: queue_rows,
            ..area
        },
        request: Rect {
            y: request_y,
            height: request_rows,
            ..area
        },
        composer: Rect {
            y: composer_y,
            height: composer_rows,
            ..area
        },
        status: Rect {
            y: status_y,
            height: status_rows,
            ..area
        },
        subagent_pane: Rect {
            y: subagent_pane_y,
            height: subagent_pane_rows,
            ..area
        },
    }
}

#[cfg(test)]
#[path = "screen_layout_tests.rs"]
mod tests;
