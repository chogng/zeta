use zui::Point;

use super::{
    CursorFeedback, ElementId, FocusBehavior, InteractionFrame, NavigationAxis, NodeAction,
    UiIntent,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DispatchInvalidation {
    #[default]
    None,
    Paint,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DispatchOutcome {
    pub invalidation: DispatchInvalidation,
    pub intent: Option<UiIntent>,
}

impl DispatchOutcome {
    const fn paint() -> Self {
        Self {
            invalidation: DispatchInvalidation::Paint,
            intent: None,
        }
    }

    const fn with_intent(intent: UiIntent, invalidation: DispatchInvalidation) -> Self {
        Self {
            invalidation,
            intent: Some(intent),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FocusDirection {
    Previous,
    Next,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UiDispatch {
    hovered_path: Vec<ElementId>,
    pressed: Option<ElementId>,
    captured: Option<ElementId>,
    focused: Option<ElementId>,
    window_active: bool,
}

impl Default for UiDispatch {
    fn default() -> Self {
        Self {
            hovered_path: Vec::new(),
            pressed: None,
            captured: None,
            focused: None,
            window_active: true,
        }
    }
}

impl UiDispatch {
    pub fn pointer_moved(&mut self, point: Point, frame: &InteractionFrame) -> DispatchOutcome {
        let hovered_path = frame
            .target_at(point)
            .map(|target| frame.ancestry(target))
            .unwrap_or_default();
        self.set_hovered_path(hovered_path)
    }

    /// Projects a host-resolved pointer target after geometry changes without another pointer
    /// event.
    ///
    /// Scroll hosts use this when they can map the retained pointer position to a stable element
    /// before rebuilding the next interaction frame. The target must exist in the active scope of
    /// `frame`; invalid targets leave the current hover path unchanged.
    pub fn hover_element(
        &mut self,
        target: ElementId,
        frame: &InteractionFrame,
    ) -> DispatchOutcome {
        if !frame.is_in_active_scope(target) || frame.node(target).is_none() {
            return DispatchOutcome::default();
        }
        self.set_hovered_path(frame.ancestry(target))
    }

    fn set_hovered_path(&mut self, hovered_path: Vec<ElementId>) -> DispatchOutcome {
        if self.hovered_path == hovered_path {
            return DispatchOutcome::default();
        }
        self.hovered_path = hovered_path;
        DispatchOutcome::paint()
    }

    pub fn pointer_left(&mut self) -> DispatchOutcome {
        if self.hovered_path.is_empty() {
            return DispatchOutcome::default();
        }
        self.hovered_path.clear();
        DispatchOutcome::paint()
    }

    pub fn press_primary(&mut self, frame: &InteractionFrame) -> DispatchOutcome {
        let Some(target) = self.hovered_path.last().copied() else {
            return DispatchOutcome::default();
        };
        if !frame.is_in_active_scope(target) {
            return DispatchOutcome::default();
        }
        let Some(node) = frame.node(target) else {
            return DispatchOutcome::default();
        };
        let focus_changed =
            node.focus_behavior() == FocusBehavior::TabStop && self.focused != Some(target);
        if node.focus_behavior() == FocusBehavior::TabStop {
            self.focused = Some(target);
        }
        match node.action() {
            NodeAction::StartWindowDrag => DispatchOutcome::with_intent(
                UiIntent::StartWindowDrag(target),
                if focus_changed {
                    DispatchInvalidation::Paint
                } else {
                    DispatchInvalidation::None
                },
            ),
            NodeAction::Activate => {
                self.pressed = Some(target);
                self.captured = Some(target);
                DispatchOutcome::paint()
            }
            NodeAction::None if focus_changed => DispatchOutcome::paint(),
            NodeAction::None => DispatchOutcome::default(),
        }
    }

    pub fn release_primary(&mut self, point: Point, frame: &InteractionFrame) -> DispatchOutcome {
        let captured = self.captured.take();
        let pressed = self.pressed.take();
        let intent = captured
            .filter(|captured| frame.target_at(point) == Some(*captured))
            .and_then(|target| {
                (frame.node(target)?.action() == NodeAction::Activate)
                    .then_some(UiIntent::Activate(target))
            });
        match (pressed, intent) {
            (Some(_), Some(intent)) => {
                DispatchOutcome::with_intent(intent, DispatchInvalidation::Paint)
            }
            (Some(_), None) => DispatchOutcome::paint(),
            (None, Some(intent)) => {
                DispatchOutcome::with_intent(intent, DispatchInvalidation::None)
            }
            (None, None) => DispatchOutcome::default(),
        }
    }

    pub fn reconcile_focus(
        &mut self,
        frame: &InteractionFrame,
        preferred: ElementId,
    ) -> DispatchOutcome {
        let focused_is_valid = self.focused.is_some_and(|focused| {
            frame.is_in_active_scope(focused)
                && frame
                    .node(focused)
                    .is_some_and(|node| node.focus_behavior() == FocusBehavior::TabStop)
        });
        if focused_is_valid {
            return DispatchOutcome::default();
        }
        let next = frame
            .node(preferred)
            .filter(|node| {
                frame.is_in_active_scope(node.id())
                    && node.focus_behavior() == FocusBehavior::TabStop
            })
            .map(|node| node.id())
            .or_else(|| frame.focus_order().next());
        if self.focused == next {
            return DispatchOutcome::default();
        }
        self.focused = next;
        DispatchOutcome::paint()
    }

    pub fn focus_in_order(
        &mut self,
        frame: &InteractionFrame,
        direction: FocusDirection,
    ) -> DispatchOutcome {
        let order = frame.focus_order().collect::<Vec<_>>();
        if order.is_empty() {
            return DispatchOutcome::default();
        }
        let current = self
            .focused
            .and_then(|focused| order.iter().position(|id| *id == focused));
        let index = match (current, direction) {
            (Some(0), FocusDirection::Previous) | (None, FocusDirection::Previous) => {
                order.len() - 1
            }
            (Some(index), FocusDirection::Previous) => index - 1,
            (Some(index), FocusDirection::Next) => (index + 1) % order.len(),
            (None, FocusDirection::Next) => 0,
        };
        self.set_focus(order[index])
    }

    pub fn focus_element(&mut self, frame: &InteractionFrame, id: ElementId) -> DispatchOutcome {
        let is_focusable = frame.is_in_active_scope(id)
            && frame
                .node(id)
                .is_some_and(|node| node.focus_behavior() == FocusBehavior::TabStop);
        if is_focusable {
            self.set_focus(id)
        } else {
            DispatchOutcome::default()
        }
    }

    pub fn focus_within_group(
        &mut self,
        frame: &InteractionFrame,
        direction: FocusDirection,
        axis: NavigationAxis,
    ) -> DispatchOutcome {
        let Some(focused) = self.focused else {
            return DispatchOutcome::default();
        };
        let Some((group, focused_axis)) = frame.node(focused).and_then(|node| node.navigation())
        else {
            return DispatchOutcome::default();
        };
        if focused_axis != axis {
            return DispatchOutcome::default();
        }
        let order = frame
            .focus_order()
            .filter(|id| {
                frame
                    .node(*id)
                    .and_then(|node| node.navigation())
                    .is_some_and(|(candidate, candidate_axis)| {
                        candidate == group && candidate_axis == axis
                    })
            })
            .collect::<Vec<_>>();
        let Some(index) = order.iter().position(|id| *id == focused) else {
            return DispatchOutcome::default();
        };
        let next = match direction {
            FocusDirection::Previous if index == 0 => order.len() - 1,
            FocusDirection::Previous => index - 1,
            FocusDirection::Next => (index + 1) % order.len(),
        };
        self.set_focus(order[next])
    }

    pub fn activate_focused(&self, frame: &InteractionFrame) -> DispatchOutcome {
        let Some(target) = self.focused else {
            return DispatchOutcome::default();
        };
        if !frame.is_in_active_scope(target) {
            return DispatchOutcome::default();
        }
        match frame.node(target).map(|node| node.action()) {
            Some(NodeAction::Activate) => DispatchOutcome::with_intent(
                UiIntent::Activate(target),
                DispatchInvalidation::Paint,
            ),
            Some(NodeAction::None | NodeAction::StartWindowDrag) | None => {
                DispatchOutcome::default()
            }
        }
    }

    pub fn window_focused(&mut self) -> DispatchOutcome {
        if self.window_active {
            return DispatchOutcome::default();
        }
        self.window_active = true;
        DispatchOutcome::paint()
    }

    pub fn window_blurred(&mut self) -> DispatchOutcome {
        let had_visual_state = self.window_active
            || !self.hovered_path.is_empty()
            || self.pressed.is_some()
            || self.captured.is_some();
        self.window_active = false;
        self.hovered_path.clear();
        self.pressed = None;
        self.captured = None;
        if had_visual_state {
            DispatchOutcome::paint()
        } else {
            DispatchOutcome::default()
        }
    }

    pub fn is_hovered(&self, id: ElementId) -> bool {
        self.hovered_path.contains(&id)
    }

    pub fn is_pressed(&self, id: ElementId) -> bool {
        self.window_active && self.pressed == Some(id) && self.hovered_path.last() == Some(&id)
    }

    pub fn is_focused(&self, id: ElementId) -> bool {
        self.window_active && matches!(self.focused, Some(focused) if focused == id)
    }

    pub const fn focused(&self) -> Option<ElementId> {
        self.focused
    }

    pub const fn window_active(&self) -> bool {
        self.window_active
    }

    pub fn pointer_feedback(&self, frame: &InteractionFrame) -> CursorFeedback {
        self.hovered_path
            .last()
            .filter(|id| frame.is_in_active_scope(**id))
            .and_then(|id| frame.node(*id))
            .map(|node| node.cursor())
            .unwrap_or_default()
    }

    fn set_focus(&mut self, id: ElementId) -> DispatchOutcome {
        if self.focused == Some(id) {
            return DispatchOutcome::default();
        }
        self.focused = Some(id);
        DispatchOutcome::paint()
    }
}
