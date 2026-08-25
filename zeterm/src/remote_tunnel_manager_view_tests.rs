use std::num::NonZeroU16;

use zeta_ui::CaretVisibility;
use zeta_ui::Rect;
use zeta_ui::TextInputLayoutEngine;
use zui::ui::AccessibilityRole;
use zui::ui::InteractionFrame;
use zui::ui::UiDispatch;
use zui::ui::UiFrame;

use super::RemoteTunnelManager;
use crate::remote_tunnel_manager::RemoteTunnelManagerState;
use crate::shell_style::SHELL_PALETTE;

#[test]
fn manager_is_modal_accessible_and_exposes_active_tunnel_controls() {
    let mut state = RemoteTunnelManagerState::default();
    state.open("build.example", None);
    state.start_succeeded(1, NonZeroU16::new(3_000).unwrap());
    let dispatch = UiDispatch::default();
    let mut text_layout = TextInputLayoutEngine::new();
    let manager = RemoteTunnelManager::new(
        Rect::from_xywh(0.0, 0.0, 1_000.0, 700.0),
        &state,
        CaretVisibility::Visible,
        SHELL_PALETTE,
        &mut text_layout,
        &dispatch,
    )
    .unwrap();
    let mut frame = UiFrame::<InteractionFrame>::new(SHELL_PALETTE.background);
    frame.draw_component(&manager);

    assert!(manager.panel_bounds().size.width <= 640.0);
    let nodes = frame.interaction().accessibility_nodes(&dispatch);
    assert_eq!(nodes[0].role, AccessibilityRole::Group);
    assert!(
        nodes
            .iter()
            .any(|node| node.role == AccessibilityRole::TextInput)
    );
    assert!(
        nodes
            .iter()
            .any(|node| node.role == AccessibilityRole::ListItem)
    );
    assert!(
        nodes
            .iter()
            .filter(|node| node.role == AccessibilityRole::Button)
            .count()
            >= 3
    );
}
