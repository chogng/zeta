use super::{InteractionLayout, bottom_anchored_area, frame_areas};
use ratatui::layout::Rect;

#[test]
fn composer_frame_surfaces_are_anchored_to_bottom() {
    let area = Rect::new(0, 0, 80, 24);

    let areas = frame_areas(area, InteractionLayout::Composer);

    assert_eq!(areas.header, Rect::new(0, 0, 80, 2));
    assert_eq!(areas.history, Rect::new(0, 2, 80, 17));
    assert_eq!(areas.status_line, Rect::new(0, 19, 80, 1));
    assert_eq!(areas.interaction, Rect::new(0, 20, 80, 3));
    assert_eq!(areas.footer, Rect::new(0, 23, 80, 1));
}

#[test]
fn expanded_interaction_grows_upward_from_bottom() {
    let area = Rect::new(0, 0, 80, 24);

    let shorter = frame_areas(area, InteractionLayout::Expanded { desired_height: 8 });
    let taller = frame_areas(area, InteractionLayout::Expanded { desired_height: 12 });

    assert_eq!(shorter.interaction.y + shorter.interaction.height, 24);
    assert_eq!(taller.interaction.y + taller.interaction.height, 24);
    assert_eq!(taller.interaction.y, shorter.interaction.y - 4);
    assert_eq!(taller.history.y, shorter.history.y);
    assert_eq!(taller.history.height + 4, shorter.history.height);
    assert_eq!(shorter.footer.height, 0);
    assert_eq!(taller.footer.height, 0);
    assert_eq!(shorter.status_line.height, 0);
    assert_eq!(taller.status_line.height, 0);
}

#[test]
fn oversized_interaction_preserves_minimum_history_height() {
    let area = Rect::new(0, 0, 80, 24);

    let areas = frame_areas(area, InteractionLayout::Expanded { desired_height: 99 });

    assert_eq!(areas.history, Rect::new(0, 2, 80, 4));
    assert_eq!(areas.status_line.height, 0);
    assert_eq!(areas.interaction, Rect::new(0, 6, 80, 18));
}

#[test]
fn bottom_anchor_respects_nonzero_terminal_origin() {
    let area = Rect::new(5, 7, 80, 24);

    let areas = frame_areas(area, InteractionLayout::Expanded { desired_height: 12 });

    assert_eq!(areas.header, Rect::new(5, 7, 80, 2));
    assert_eq!(areas.history, Rect::new(5, 9, 80, 10));
    assert_eq!(areas.status_line, Rect::new(5, 19, 80, 0));
    assert_eq!(areas.interaction, Rect::new(5, 19, 80, 12));
    assert_eq!(areas.footer, Rect::new(5, 31, 80, 0));
}

#[test]
fn popup_area_is_clamped_and_anchored_to_its_parent_bottom() {
    let parent = Rect::new(5, 7, 80, 6);

    assert_eq!(bottom_anchored_area(parent, 4), Rect::new(5, 9, 80, 4));
    assert_eq!(bottom_anchored_area(parent, 99), parent);
}
