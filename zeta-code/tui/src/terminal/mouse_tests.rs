use super::PointerInteraction;

#[test]
fn pointer_hover_is_transient_and_has_no_selection_side_effect() {
    let mut pointer = PointerInteraction::default();

    pointer.update_hover(Some("second"));
    assert_eq!(pointer.hovered(), Some(&"second"));
    pointer.update_pressed(Some("second"));
    assert_eq!(pointer.pressed(), Some(&"second"));

    pointer.clear_pressed();
    assert_eq!(pointer.hovered(), Some(&"second"));
    assert_eq!(pointer.pressed(), None);
    pointer.clear();
    assert_eq!(pointer.pressed(), None);
}
