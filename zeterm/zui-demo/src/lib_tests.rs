use super::{build_demo_frame, render_demo};

#[test]
fn demo_composes_reusable_components_without_a_product_host() {
    let frame = build_demo_frame();

    assert_eq!(frame.scene().rects().len(), 1);
    assert_eq!(frame.scene().icons().len(), 1);
    assert_eq!(frame.scene().text_blocks().len(), 1);
    assert!(!frame.scene().inspection().nodes().is_empty());
}

#[test]
fn demo_can_be_submitted_to_a_replaceable_renderer_boundary() {
    let stats = render_demo().expect("recording renderer should present the demo scene");

    assert_eq!(stats.scene_count, 1);
    assert_eq!(stats.rect_count, 1);
    assert_eq!(stats.icon_count, 1);
    assert_eq!(stats.text_count, 1);
}
