use super::manager_areas;
use super::session_areas;
use ratatui::layout::Rect;

#[test]
fn session_layout_bounds_queue_and_preserves_transcript() {
    let areas = session_areas(Rect::new(0, 0, 80, 20), 1, 1, 12, 0, 3, 2, 4);

    assert_eq!(areas.transcript.height, 4);
    assert_eq!(areas.goal.height, 0);
    assert_eq!(areas.plan.height, 0);
    assert_eq!(areas.queue.height, 6);
    assert_eq!(areas.composer.height, 3);
    assert_eq!(areas.status.height, 2);
    assert_eq!(areas.subagent_picker.height, 4);
}

#[test]
fn session_layout_uses_zero_height_for_absent_rows() {
    let areas = session_areas(Rect::new(0, 0, 80, 20), 0, 0, 0, 0, 3, 1, 0);

    assert_eq!(areas.queue.height, 0);
    assert_eq!(areas.composer.y, areas.transcript.height);
}

#[test]
fn session_layout_places_goal_plan_and_queue_above_input() {
    let areas = session_areas(Rect::new(0, 0, 80, 20), 1, 1, 2, 0, 3, 1, 2);

    assert_eq!(areas.goal.y, areas.transcript.height);
    assert_eq!(areas.plan.y, areas.goal.y + areas.goal.height);
    assert_eq!(areas.queue.y, areas.plan.y + areas.plan.height);
    assert_eq!(areas.composer.y, areas.queue.y + areas.queue.height);
    assert_eq!(areas.status.y, areas.composer.y + areas.composer.height);
    assert_eq!(
        areas.subagent_picker.y,
        areas.status.y + areas.status.height + 1
    );
}

#[test]
fn session_layout_does_not_reserve_a_subagent_gap_without_both_surfaces() {
    let without_subagents = session_areas(Rect::new(0, 0, 80, 20), 0, 0, 0, 0, 3, 1, 0);
    let without_status = session_areas(Rect::new(0, 0, 80, 20), 0, 0, 0, 0, 3, 0, 2);

    assert_eq!(
        without_subagents.subagent_picker.y,
        without_subagents.status.y + without_subagents.status.height
    );
    assert_eq!(
        without_status.subagent_picker.y,
        without_status.status.y + without_status.status.height
    );
}

#[test]
fn session_layout_places_query_immediately_above_the_input_area() {
    let areas = session_areas(Rect::new(0, 0, 80, 20), 0, 0, 0, 1, 3, 1, 0);

    assert_eq!(areas.request.height, 1);
    assert_eq!(areas.composer.y, areas.request.y + areas.request.height);
}

#[test]
fn manager_layout_keeps_welcome_above_a_useful_session_list() {
    let areas = manager_areas(Rect::new(0, 2, 80, 20), 11);

    assert_eq!(areas.welcome, Rect::new(0, 2, 80, 11));
    assert_eq!(areas.sessions, Rect::new(0, 14, 80, 8));
}

#[test]
fn manager_layout_shrinks_welcome_before_the_session_list() {
    let areas = manager_areas(Rect::new(0, 0, 40, 8), 12);

    assert_eq!(areas.welcome.height, 3);
    assert_eq!(areas.sessions, Rect::new(0, 4, 40, 4));
}
