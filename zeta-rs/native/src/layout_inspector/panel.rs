use zeta_ui::{
    Border, Color, Component, ComponentInspection, Edges, PaintRect, Point, Rect, UiScene,
};

use super::InspectionSelection;
use super::inspector_content::{InspectorContent, InspectorContentState};
use super::inspector_toolbar::{InspectorToolbar, InspectorToolbarAction, InspectorToolbarState};

const PANEL_BACKGROUND: Color = Color::rgb(248, 248, 250);
const PANEL_BORDER: Color = Color::rgb(218, 218, 224);

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct PanelState {
    pub(super) picking: bool,
    pub(super) pointer: Option<Point>,
}

pub(super) struct InspectorPanel<'a> {
    bounds: Rect,
    selection: Option<&'a InspectionSelection>,
    state: PanelState,
}

impl<'a> InspectorPanel<'a> {
    pub(super) const fn new(
        bounds: Rect,
        selection: Option<&'a InspectionSelection>,
        state: PanelState,
    ) -> Self {
        Self {
            bounds,
            selection,
            state,
        }
    }
}

pub(super) fn toolbar_action_at(
    panel_bounds: Rect,
    point: Point,
) -> Option<InspectorToolbarAction> {
    InspectorToolbar::hit_test(InspectorToolbar::bounds(panel_bounds), point)
}

pub(super) fn row_index_at(panel_bounds: Rect, point: Point, row_count: usize) -> Option<usize> {
    let toolbar_bounds = InspectorToolbar::bounds(panel_bounds);
    InspectorContent::row_index_at(
        content_bounds(panel_bounds, toolbar_bounds),
        point,
        row_count,
    )
}

impl Component for InspectorPanel<'_> {
    fn inspection(&self) -> ComponentInspection {
        ComponentInspection::new("InspectorPanel", self.bounds)
    }

    fn paint(&self, scene: &mut UiScene) {
        if self.bounds.is_empty() {
            return;
        }
        scene.with_clip(self.bounds, |scene| {
            scene.draw_rect(
                PaintRect::new(self.bounds, PANEL_BACKGROUND)
                    .with_border(Border::new(Edges::new(0.0, 0.0, 0.0, 1.0), PANEL_BORDER)),
            );
            let toolbar_bounds = InspectorToolbar::bounds(self.bounds);
            let hovered_action = self
                .state
                .pointer
                .and_then(|point| InspectorToolbar::hit_test(toolbar_bounds, point));
            scene.draw_component(&InspectorToolbar::new(
                toolbar_bounds,
                InspectorToolbarState {
                    picking: self.state.picking,
                    hovered: hovered_action,
                },
            ));

            let content_bounds = content_bounds(self.bounds, toolbar_bounds);
            let hovered_row = self.state.pointer.and_then(|point| {
                InspectorContent::row_index_at(
                    content_bounds,
                    point,
                    self.selection.map_or(0, |selection| selection.path.len()),
                )
            });
            scene.draw_component(&InspectorContent::new(
                content_bounds,
                self.selection,
                InspectorContentState {
                    picking: self.state.picking,
                    hovered_row,
                },
            ));
        });
    }
}

fn content_bounds(panel_bounds: Rect, toolbar_bounds: Rect) -> Rect {
    Rect::from_xywh(
        panel_bounds.origin.x,
        toolbar_bounds.bottom(),
        panel_bounds.size.width,
        (panel_bounds.size.height - toolbar_bounds.size.height).max(0.0),
    )
}
