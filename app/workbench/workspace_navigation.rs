use zeta_ui_components::ActionBar;
use zeta_ui_components::ActionBarItem;
use zeta_ui_components::ActionBarOrientation;
use zeta_ui_components::ActionBarStyle;
use zeta_ui_components::ActionViewItem;
use zeta_ui_components::ButtonSelection;
use zeta_ui_components::ButtonState;
use zeta_ui_components::InteractionRegion;
use zui::ui::AccessibilityRole;
use zui::ui::AccessibilitySelection;
use zui::ui::Component;
use zui::ui::ComponentContext;
use zui::ui::ComponentElement;
use zui::ui::ComputedElement;
use zui::ui::CursorFeedback;
use zui::ui::Element;
use zui::ui::ElementId;
use zui::ui::FocusBehavior;
use zui::ui::NavigationAxis;
use zui::ui::NavigationGroupId;
use zui::ui::NodeAction;
use zui::ui::Rect;
use zui::ui::Size;
use zui::ui::UiDispatch;
use zui::ui::UiNode;
use zui::ui::UiScene;

use crate::PaneInputKind;
use crate::WorkspaceNavigationStyle;

const ITEM_WIDTH: f32 = 64.0;
pub const WORKSPACE_PANE: ElementId = ElementId::scoped(1, 23);
pub const WORKSPACE_PANE_NAVIGATION: ElementId = ElementId::scoped(1, 32);
pub const WORKSPACE_CHANGES: ElementId = ElementId::scoped(1, 33);
pub const WORKSPACE_FILES: ElementId = ElementId::scoped(1, 34);
pub const WORKSPACE_PANE_TOOLBAR: ElementId = ElementId::scoped(1, 35);

/// A concrete capability selectable in the Workspace Pane.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkspacePaneSelection {
    Changes,
    Files,
}

impl WorkspacePaneSelection {
    pub const ALL: [Self; 2] = [Self::Changes, Self::Files];

    pub const fn element_id(self) -> ElementId {
        match self {
            Self::Changes => WORKSPACE_CHANGES,
            Self::Files => WORKSPACE_FILES,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Changes => "Changes",
            Self::Files => "Files",
        }
    }

    pub const fn pane_kind(self) -> PaneInputKind {
        match self {
            Self::Changes => PaneInputKind::Diff,
            Self::Files => PaneInputKind::Files,
        }
    }

    pub const fn from_pane_kind(kind: PaneInputKind) -> Option<Self> {
        match kind {
            PaneInputKind::Diff => Some(Self::Changes),
            PaneInputKind::Files => Some(Self::Files),
            PaneInputKind::Agent | PaneInputKind::Settings | PaneInputKind::Terminal => None,
        }
    }

    pub const fn from_element_id(id: ElementId) -> Option<Self> {
        match id {
            WORKSPACE_CHANGES => Some(Self::Changes),
            WORKSPACE_FILES => Some(Self::Files),
            _ => None,
        }
    }
}

/// Horizontal Changes/Files switcher hosted by a Workspace Pane toolbar.
pub struct WorkspacePaneNavigation {
    bounds: Rect,
    action_bar: ActionBar,
    selected: WorkspacePaneSelection,
}

impl WorkspacePaneNavigation {
    pub fn bounds_in(toolbar: Rect) -> Rect {
        Rect::from_xywh(
            toolbar.origin.x,
            toolbar.origin.y,
            (ITEM_WIDTH * WorkspacePaneSelection::ALL.len() as f32).min(toolbar.size.width),
            toolbar.size.height,
        )
    }

    pub fn new(
        bounds: Rect,
        selected: WorkspacePaneSelection,
        palette: &WorkspaceNavigationStyle,
        dispatch: &UiDispatch,
    ) -> Self {
        let button_style = palette.button_style();
        let items = WorkspacePaneSelection::ALL
            .into_iter()
            .map(|action| {
                let target = action.element_id();
                let state = if dispatch.is_pressed(target) {
                    ButtonState::Pressed
                } else if dispatch.is_focused(target) {
                    ButtonState::Focused
                } else if dispatch.is_hovered(target) {
                    ButtonState::Hovered
                } else {
                    ButtonState::Resting
                };
                ActionBarItem::Action(ActionViewItem::label(action.label(), state).with_selection(
                    if action == selected {
                        ButtonSelection::Selected
                    } else {
                        ButtonSelection::Unselected
                    },
                ))
            })
            .collect();
        Self {
            bounds,
            selected,
            action_bar: ActionBar::new(
                bounds,
                ActionBarOrientation::Horizontal,
                items,
                ActionBarStyle::new(button_style, Size::new(ITEM_WIDTH, bounds.size.height)),
            ),
        }
    }

    fn child_interaction_regions(&self) -> Vec<InteractionRegion> {
        let mut regions = Vec::new();
        let navigation = NavigationGroupId::new(WORKSPACE_PANE_NAVIGATION);
        for (index, action) in WorkspacePaneSelection::ALL.into_iter().enumerate() {
            let bounds = self
                .action_bar
                .interactive_item_bounds(index)
                .expect("pane actions are enabled");
            regions.push(
                InteractionRegion::new(
                    "WorkspacePaneButton",
                    action.element_id(),
                    bounds,
                    AccessibilityRole::Button,
                    action.label(),
                )
                .with_cursor(CursorFeedback::Pointer)
                .with_focus(FocusBehavior::TabStop)
                .with_action(NodeAction::Activate)
                .with_navigation(navigation, NavigationAxis::Horizontal)
                .with_selection(if action == self.selected {
                    AccessibilitySelection::Selected
                } else {
                    AccessibilitySelection::Unselected
                }),
            );
        }
        regions
    }
}

impl Component for WorkspacePaneNavigation {
    fn element(&self) -> ComponentElement {
        Element::leaf("WorkspacePaneNavigation")
            .in_bounds(self.bounds)
            .with_identity(WORKSPACE_PANE_NAVIGATION)
    }

    fn interaction_node(&self, element: &ComputedElement) -> Option<UiNode> {
        Some(
            UiNode::new(
                WORKSPACE_PANE_NAVIGATION,
                element.bounds(),
                AccessibilityRole::Toolbar,
                "Workspace panes",
            )
            .with_parent(WORKSPACE_PANE_TOOLBAR),
        )
    }

    fn compose(&self, context: &mut ComponentContext<'_, '_>, _element: &ComputedElement) {
        for region in self.child_interaction_regions() {
            context.draw_component(&region);
        }
        context.draw_component(&self.action_bar);
    }

    fn paint(&self, scene: &mut UiScene) {
        scene.draw_component(&self.action_bar);
    }
}

#[cfg(test)]
#[path = "workspace_navigation_tests.rs"]
mod tests;
