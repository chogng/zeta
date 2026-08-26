use std::sync::Arc;
use std::sync::Mutex;

use zui::ui::ComponentRuntime;
use zui::ui::ViewState;

use super::DEMO_VIEW;
use super::build_demo_frame;
use super::build_demo_frame_with_state;
use super::render_demo;

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

#[test]
fn demo_view_state_invalidates_its_retained_component() {
    let invalidated = Arc::new(Mutex::new(Vec::new()));
    let observed = invalidated.clone();
    let mut runtime =
        ComponentRuntime::new(move |component| observed.lock().unwrap().push(component));
    let ready = ViewState::new(false);
    let _frame = build_demo_frame_with_state(&ready, &mut runtime);

    ready.update(|ready| *ready = true);

    assert_eq!(*invalidated.lock().unwrap(), vec![DEMO_VIEW]);
}
