use super::PointerInteraction;

#[test]
fn pointer_hover_is_transient_and_has_no_selection_side_effect() {
    let mut pointer = PointerInteraction::default();

    pointer.update_hover(Some("second"));
    assert_eq!(pointer.hovered(), Some(&"second"));

    pointer.clear_hover();
    assert_eq!(pointer.hovered(), None);
}
