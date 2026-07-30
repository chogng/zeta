use zeta_ui::{Point, Rect};

use super::{
    AccessibilityRole, AccessibilitySelection, CursorFeedback, ElementId, FocusBehavior,
    NavigationAxis, NavigationGroupId, NodeAction, UiDispatch,
};

#[derive(Clone, Debug, PartialEq)]
pub struct UiNode {
    id: ElementId,
    parent: Option<ElementId>,
    bounds: Rect,
    cursor: CursorFeedback,
    focus: FocusBehavior,
    action: NodeAction,
    navigation: Option<(NavigationGroupId, NavigationAxis)>,
    role: AccessibilityRole,
    label: String,
    value: Option<String>,
    selection: AccessibilitySelection,
}

impl UiNode {
    pub fn new(
        id: ElementId,
        bounds: Rect,
        role: AccessibilityRole,
        label: impl Into<String>,
    ) -> Self {
        Self {
            id,
            parent: None,
            bounds,
            cursor: CursorFeedback::Default,
            focus: FocusBehavior::None,
            action: NodeAction::None,
            navigation: None,
            role,
            label: label.into(),
            value: None,
            selection: AccessibilitySelection::NotApplicable,
        }
    }

    pub const fn with_parent(mut self, parent: ElementId) -> Self {
        self.parent = Some(parent);
        self
    }

    pub const fn with_cursor(mut self, cursor: CursorFeedback) -> Self {
        self.cursor = cursor;
        self
    }

    pub const fn with_focus(mut self, focus: FocusBehavior) -> Self {
        self.focus = focus;
        self
    }

    pub const fn with_action(mut self, action: NodeAction) -> Self {
        self.action = action;
        self
    }

    pub const fn with_navigation(mut self, group: NavigationGroupId, axis: NavigationAxis) -> Self {
        self.navigation = Some((group, axis));
        self
    }

    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    pub const fn with_selection(mut self, selection: AccessibilitySelection) -> Self {
        self.selection = selection;
        self
    }

    pub const fn id(&self) -> ElementId {
        self.id
    }

    pub const fn parent(&self) -> Option<ElementId> {
        self.parent
    }

    pub const fn cursor(&self) -> CursorFeedback {
        self.cursor
    }

    pub const fn focus_behavior(&self) -> FocusBehavior {
        self.focus
    }

    pub const fn action(&self) -> NodeAction {
        self.action
    }

    pub const fn navigation(&self) -> Option<(NavigationGroupId, NavigationAxis)> {
        self.navigation
    }

    fn contains(&self, point: Point) -> bool {
        self.bounds.contains(point)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct InteractionFrame {
    nodes: Vec<UiNode>,
}

impl InteractionFrame {
    pub fn register(&mut self, node: UiNode) {
        debug_assert!(
            self.node(node.id()).is_none(),
            "an element may only be registered once per frame"
        );
        self.nodes.push(node);
    }

    pub fn node(&self, id: ElementId) -> Option<&UiNode> {
        self.nodes.iter().find(|node| node.id == id)
    }

    pub fn target_at(&self, point: Point) -> Option<ElementId> {
        self.nodes
            .iter()
            .rev()
            .find(|node| node.contains(point))
            .map(UiNode::id)
    }

    pub fn ancestry(&self, id: ElementId) -> Vec<ElementId> {
        let mut path = Vec::new();
        let mut current = Some(id);
        while let Some(id) = current {
            path.push(id);
            current = self.node(id).and_then(UiNode::parent);
        }
        path.reverse();
        path
    }

    pub fn focus_order(&self) -> impl Iterator<Item = ElementId> + '_ {
        self.nodes.iter().filter_map(|node| {
            (node.focus_behavior() == FocusBehavior::TabStop).then_some(node.id())
        })
    }

    pub fn accessibility_nodes(&self, dispatch: &UiDispatch) -> Vec<AccessibilityNode> {
        self.nodes
            .iter()
            .map(|node| AccessibilityNode {
                id: node.id,
                parent: node.parent,
                role: node.role,
                label: node.label.clone(),
                value: node.value.clone(),
                selection: node.selection,
                bounds: node.bounds,
                focusable: node.focus == FocusBehavior::TabStop,
                focused: dispatch.is_focused(node.id),
            })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AccessibilityNode {
    pub id: ElementId,
    pub parent: Option<ElementId>,
    pub role: AccessibilityRole,
    pub label: String,
    pub value: Option<String>,
    pub bounds: Rect,
    pub focusable: bool,
    pub focused: bool,
    pub selection: AccessibilitySelection,
}
