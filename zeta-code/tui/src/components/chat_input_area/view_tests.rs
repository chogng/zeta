use super::areas;
use super::desired_height;
use crate::components::chat_input_area::ChatInputAreaHeightEntryKind;
use ratatui::layout::Rect;

#[test]
fn pane_stacks_above_the_persistent_input() {
    let entries = [(ChatInputAreaHeightEntryKind::Pane, 9)];
    let areas = areas(Rect::new(0, 10, 80, 12), 3, &entries);

    assert_eq!(areas.height_entries[0].area, Rect::new(0, 10, 80, 9));
    assert_eq!(areas.input, Rect::new(0, 19, 80, 3));
    assert_eq!(desired_height(3, &entries), 12);
}

#[test]
fn input_remains_visible_when_the_pane_is_clamped() {
    let entries = [(ChatInputAreaHeightEntryKind::Pane, 99)];
    let areas = areas(Rect::new(0, 10, 80, 7), 3, &entries);

    assert_eq!(areas.height_entries[0].area, Rect::new(0, 10, 80, 4));
    assert_eq!(areas.input, Rect::new(0, 14, 80, 3));
}

#[test]
fn independently_sized_entries_stack_in_insertion_order() {
    let entries = [
        (ChatInputAreaHeightEntryKind::PlanProgress, 3),
        (ChatInputAreaHeightEntryKind::Queue, 4),
        (ChatInputAreaHeightEntryKind::Steer, 3),
    ];

    let areas = areas(Rect::new(0, 0, 80, 13), 3, &entries);

    assert_eq!(areas.height_entries[0].area, Rect::new(0, 7, 80, 3));
    assert_eq!(areas.height_entries[1].area, Rect::new(0, 3, 80, 4));
    assert_eq!(areas.height_entries[2].area, Rect::new(0, 0, 80, 3));
    assert_eq!(areas.input, Rect::new(0, 10, 80, 3));
}
