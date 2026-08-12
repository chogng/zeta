use super::{InteractionLayout, bottom_anchored_area, frame_areas};
use ratatui::layout::Rect;

#[test]
fn composer_frame_surfaces_are_anchored_to_bottom() {
    let area = Rect::new(0, 0, 80, 24);

    let areas = frame_areas(area, InteractionLayout::Composer { desired_height: 3 });

    assert_eq!(areas.history, Rect::new(0, 0, 80, 19));
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
fn multiline_composer_grows_upward_without_displacing_the_footer() {
    let area = Rect::new(0, 0, 80, 24);

    let areas = frame_areas(area, InteractionLayout::Composer { desired_height: 6 });

    assert_eq!(areas.history, Rect::new(0, 0, 80, 16));
    assert_eq!(areas.status_line, Rect::new(0, 16, 80, 1));
    assert_eq!(areas.interaction, Rect::new(0, 17, 80, 6));
    assert_eq!(areas.footer, Rect::new(0, 23, 80, 1));
}

#[test]
fn oversized_interaction_preserves_minimum_history_height() {
    let area = Rect::new(0, 0, 80, 24);

    let areas = frame_areas(area, InteractionLayout::Expanded { desired_height: 99 });

    assert_eq!(areas.history, Rect::new(0, 0, 80, 4));
    assert_eq!(areas.status_line.height, 0);
    assert_eq!(areas.interaction, Rect::new(0, 4, 80, 20));
}

#[test]
fn bottom_anchor_respects_nonzero_terminal_origin() {
    let area = Rect::new(5, 7, 80, 24);

    let areas = frame_areas(area, InteractionLayout::Expanded { desired_height: 12 });

    assert_eq!(areas.history, Rect::new(5, 7, 80, 12));
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
