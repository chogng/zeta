use ratatui::layout::Rect;

const MIN_TRANSCRIPT_ROWS: u16 = 4;
const MIN_MANAGER_ROWS: u16 = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ManagerAreas {
    pub(crate) welcome: Rect,
    pub(crate) sessions: Rect,
}

pub(crate) fn manager_areas(area: Rect, welcome_desired_rows: u16) -> ManagerAreas {
    let sessions_rows = MIN_MANAGER_ROWS.min(area.height);
    let available_above_sessions = area.height.saturating_sub(sessions_rows);
    let gap_rows = u16::from(available_above_sessions > 0);
    let welcome_rows = welcome_desired_rows.min(available_above_sessions.saturating_sub(gap_rows));
    let sessions_y = area.y.saturating_add(welcome_rows).saturating_add(gap_rows);
    ManagerAreas {
        welcome: Rect {
            height: welcome_rows,
            ..area
        },
        sessions: Rect {
            y: sessions_y,
            height: area
                .y
                .saturating_add(area.height)
                .saturating_sub(sessions_y),
            ..area
        },
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SessionAreas {
    pub(crate) transcript: Rect,
    pub(crate) goal: Rect,
    pub(crate) plan: Rect,
    pub(crate) queue: Rect,
    pub(crate) request: Rect,
    pub(crate) composer: Rect,
    pub(crate) status: Rect,
    pub(crate) subagent_picker: Rect,
}

pub(crate) fn session_areas(
    area: Rect,
    goal_desired_rows: u16,
    plan_desired_rows: u16,
    queue_desired_rows: u16,
    request_desired_rows: u16,
    composer_desired_rows: u16,
    status_desired_rows: u16,
    subagent_picker_desired_rows: u16,
) -> SessionAreas {
    let subagent_picker_rows = subagent_picker_desired_rows.min(area.height);
    let available_above_subagent_picker = area.height.saturating_sub(subagent_picker_rows);
    let status_rows = status_desired_rows.min(available_above_subagent_picker);
    let available_above_status = available_above_subagent_picker.saturating_sub(status_rows);
    let subagent_gap_rows =
        u16::from(subagent_picker_rows > 0 && status_rows > 0).min(available_above_status);
    let available_above_gap = available_above_status.saturating_sub(subagent_gap_rows);
    let transcript_rows = MIN_TRANSCRIPT_ROWS.min(available_above_gap);
    let available_chrome = available_above_gap.saturating_sub(transcript_rows);
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
    let subagent_picker_y = bottom.saturating_sub(subagent_picker_rows);
    let status_y = subagent_picker_y
        .saturating_sub(subagent_gap_rows)
        .saturating_sub(status_rows);
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
        subagent_picker: Rect {
            y: subagent_picker_y,
            height: subagent_picker_rows,
            ..area
        },
    }
}

#[cfg(test)]
#[path = "screen_layout_tests.rs"]
mod tests;
