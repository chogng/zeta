use accesskit::Action;
use accesskit::ActionRequest;
use accesskit::NodeId;
use accesskit::TreeId;

use super::AccessibilityActionKind;
use super::AccessibilitySnapshot;
use super::requested_action;
use crate::runtime::AccessibilityNode;
use crate::ui::foundation::AccessibilityExpansion;
use crate::ui::foundation::AccessibilityRole;
use crate::ui::foundation::AccessibilitySelection;
use crate::ui::foundation::ElementId;
use crate::ui::foundation::NodeAction;
use crate::ui::foundation::Rect;
use crate::window::WindowId;

fn button(id: ElementId, parent: Option<ElementId>) -> AccessibilityNode {
    AccessibilityNode {
        id,
        parent,
        role: AccessibilityRole::Button,
        label: "Run".to_owned(),
        value: None,
        bounds: Rect::from_xywh(10.0, 20.0, 80.0, 30.0),
        focusable: true,
        focused: true,
        action: NodeAction::Activate,
        selection: AccessibilitySelection::NotApplicable,
        level: None,
        expansion: AccessibilityExpansion::NotApplicable,
    }
}

#[test]
fn full_tree_preserves_hierarchy_focus_bounds_and_actions() {
    let group_id = ElementId::scoped(1, 1);
    let button_id = ElementId::scoped(1, 2);
    let mut group = button(group_id, None);
    group.role = AccessibilityRole::Group;
    group.action = NodeAction::None;
    group.focusable = false;
    group.focused = false;
    let snapshot = AccessibilitySnapshot::new(
        "Test window".to_owned(),
        vec![group, button(button_id, Some(group_id))],
    );

    let update = snapshot.full_update();
    assert_eq!(update.tree.as_ref().unwrap().root, snapshot.root);
    assert_eq!(update.focus, NodeId(button_id.into_raw()));
    let root = &update.nodes[0].1;
    assert_eq!(root.label(), Some("Test window"));
    assert_eq!(root.children(), &[NodeId(group_id.into_raw())]);
    let group = &update.nodes[1].1;
    assert_eq!(group.children(), &[NodeId(button_id.into_raw())]);
    let button = &update.nodes[2].1;
    assert!(button.supports_action(Action::Focus));
    assert!(button.supports_action(Action::Click));
    assert_eq!(button.bounds().unwrap().x0, 10.0);
}

#[test]
fn only_advertised_actions_target_real_snapshot_nodes() {
    let target = ElementId::scoped(4, 9);
    let snapshot = AccessibilitySnapshot::new("Test window".to_owned(), vec![button(target, None)]);
    let window = WindowId::from_raw(7);
    let action = requested_action(
        window,
        ActionRequest {
            action: Action::Click,
            target_tree: TreeId::ROOT,
            target_node: NodeId(target.into_raw()),
            data: None,
        },
        &snapshot,
    )
    .unwrap();
    assert_eq!(action.window(), window);
    assert_eq!(action.target(), target);
    assert_eq!(action.kind(), AccessibilityActionKind::Activate);

    assert!(
        requested_action(
            window,
            ActionRequest {
                action: Action::SetValue,
                target_tree: TreeId::ROOT,
                target_node: NodeId(target.into_raw()),
                data: None,
            },
            &snapshot,
        )
        .is_none()
    );
}
