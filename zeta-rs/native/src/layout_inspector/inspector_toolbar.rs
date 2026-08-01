use zeta_icons::icons;
use zeta_ui::{
    ActionBar, ActionBarButton, ActionBarItem, ActionBarOrientation, ActionBarStyle,
    ButtonBackgrounds, ButtonSelection, ButtonState, ButtonStyle, Color, Component,
    ComponentInspection, CornerRadii, Edges, PaintRect, Point, Rect, Size, TextStyle, UiScene,
};

use crate::titlebar::TITLEBAR_HEIGHT;

const BACKGROUND: Color = Color::rgb(248, 248, 250);
const BORDER: Color = Color::rgb(218, 218, 224);
const FOREGROUND: Color = Color::rgb(35, 35, 42);
const ACTION_SIZE: f32 = 24.0;
const ACTION_INSET: f32 = 8.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum InspectorToolbarAction {
    Pick,
    Close,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct InspectorToolbarState {
    pub(super) picking: bool,
    pub(super) hovered: Option<InspectorToolbarAction>,
}

pub(super) struct InspectorToolbar {
    bounds: Rect,
    leading_action_bar: ActionBar,
    trailing_action_bar: ActionBar,
}

impl InspectorToolbar {
    pub(super) fn new(bounds: Rect, state: InspectorToolbarState) -> Self {
        Self {
            bounds,
            leading_action_bar: action_bar(
                action_bounds(bounds, InspectorToolbarAction::Pick),
                if state.picking {
                    icons::CURSOR_FILLED
                } else {
                    icons::CURSOR
                },
                if state.picking {
                    "Stop selecting components"
                } else {
                    "Select a component"
                },
                button_state(state.hovered == Some(InspectorToolbarAction::Pick)),
                if state.picking {
                    ButtonSelection::Selected
                } else {
                    ButtonSelection::Unselected
                },
            ),
            trailing_action_bar: action_bar(
                action_bounds(bounds, InspectorToolbarAction::Close),
                icons::CLOSE,
                "Close layout inspector",
                button_state(state.hovered == Some(InspectorToolbarAction::Close)),
                ButtonSelection::Unselected,
            ),
        }
    }

    pub(super) fn bounds(panel_bounds: Rect) -> Rect {
        Rect::from_xywh(
            panel_bounds.origin.x,
            panel_bounds.origin.y,
            panel_bounds.size.width,
            TITLEBAR_HEIGHT.min(panel_bounds.size.height),
        )
    }

    pub(super) fn hit_test(bounds: Rect, point: Point) -> Option<InspectorToolbarAction> {
        [InspectorToolbarAction::Pick, InspectorToolbarAction::Close]
            .into_iter()
            .find(|action| action_bounds(bounds, *action).contains(point))
    }
}

impl Component for InspectorToolbar {
    fn inspection(&self) -> ComponentInspection {
        ComponentInspection::new("InspectorToolbar", self.bounds)
    }

    fn paint(&self, scene: &mut UiScene) {
        scene.draw_rect(PaintRect::new(self.bounds, BACKGROUND));
        scene.draw_rect(PaintRect::new(
            Rect::from_xywh(
                self.bounds.origin.x,
                self.bounds.bottom() - 1.0,
                self.bounds.size.width,
                1.0,
            ),
            BORDER,
        ));
        scene.draw_component(&self.leading_action_bar);
        scene.draw_component(&self.trailing_action_bar);
    }
}

fn action_bounds(toolbar_bounds: Rect, action: InspectorToolbarAction) -> Rect {
    let x = match action {
        InspectorToolbarAction::Pick => toolbar_bounds.origin.x + ACTION_INSET,
        InspectorToolbarAction::Close => toolbar_bounds.right() - ACTION_INSET - ACTION_SIZE,
    };
    Rect::from_xywh(
        x,
        toolbar_bounds.origin.y + (toolbar_bounds.size.height - ACTION_SIZE) / 2.0,
        ACTION_SIZE,
        ACTION_SIZE,
    )
}

fn action_bar(
    bounds: Rect,
    icon: zeta_icons::Icon,
    accessible_label: &'static str,
    state: ButtonState,
    selection: ButtonSelection,
) -> ActionBar {
    ActionBar::new(
        bounds,
        ActionBarOrientation::Horizontal,
        vec![ActionBarItem::Button(
            ActionBarButton::icon(icon, accessible_label, state).with_selection(selection),
        )],
        ActionBarStyle::new(button_style(), Size::new(ACTION_SIZE, ACTION_SIZE)),
    )
}

fn button_state(hovered: bool) -> ButtonState {
    if hovered {
        ButtonState::Hovered
    } else {
        ButtonState::Resting
    }
}

fn button_style() -> ButtonStyle {
    ButtonStyle::new(
        ButtonBackgrounds::new(Color::TRANSPARENT).with_hovered(Color::rgba(35, 131, 226, 24)),
        TextStyle::new(12.0, FOREGROUND),
    )
    .with_selected_backgrounds(ButtonBackgrounds::new(Color::rgba(35, 131, 226, 40)))
    .with_corner_radii(CornerRadii::uniform(4.0))
    .with_padding(Edges::uniform(6.0))
    .with_icon_size(16.0)
}
