use zeta_ui::{
    GridLayout, GridNode, GridPane, Rect, SplitViewLayoutPriority, SplitViewOrientation,
    SplitViewPane,
};

use crate::shell_scene::LogicalViewport;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum RootLeafId {
    Product,
    Inspector,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum RootSplitId {
    ProductAndInspector,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) enum InspectorPane {
    #[default]
    Hidden,
    Visible {
        preferred_width: f32,
    },
}

impl InspectorPane {
    pub(crate) const fn visible(preferred_width: f32) -> Self {
        Self::Visible { preferred_width }
    }
}

/// Top-level product and developer-tool leaf geometry for one native window frame.
///
/// The product leaf keeps its caller-provided viewport width. When the native window has gained
/// additional width, the Inspector contributes a sibling Grid leaf instead of consuming space
/// inside the product leaf.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RootLayout {
    product_bounds: Rect,
    inspector_bounds: Option<Rect>,
}

impl RootLayout {
    pub(crate) fn for_viewports(
        window_viewport: LogicalViewport,
        product_viewport: LogicalViewport,
        inspector: InspectorPane,
    ) -> Self {
        let inspector_width = match inspector {
            InspectorPane::Hidden => 0.0,
            InspectorPane::Visible { preferred_width } => (window_viewport.width
                - product_viewport.width)
                .max(0.0)
                .min(preferred_width.max(0.0)),
        };
        let root_bounds = Rect::from_xywh(
            0.0,
            0.0,
            product_viewport.width + inspector_width,
            product_viewport.height,
        );
        if inspector_width <= 0.0 {
            let layout = GridLayout::new(
                root_bounds,
                &GridNode::<RootLeafId, RootSplitId>::leaf(RootLeafId::Product),
            );
            return Self {
                product_bounds: layout
                    .leaf(RootLeafId::Product)
                    .expect("Root Grid must retain its product leaf")
                    .bounds(),
                inspector_bounds: None,
            };
        }
        let root = GridNode::split(
            RootSplitId::ProductAndInspector,
            SplitViewOrientation::Horizontal,
            vec![
                GridPane::new(
                    GridNode::leaf(RootLeafId::Product),
                    exact_pane(product_viewport.width).with_priority(SplitViewLayoutPriority::High),
                ),
                GridPane::new(
                    GridNode::leaf(RootLeafId::Inspector),
                    exact_pane(inspector_width),
                ),
            ],
        );
        let layout = GridLayout::new(root_bounds, &root);
        Self {
            product_bounds: layout
                .leaf(RootLeafId::Product)
                .expect("Root Grid must retain its product leaf")
                .bounds(),
            inspector_bounds: layout.leaf(RootLeafId::Inspector).map(|leaf| leaf.bounds()),
        }
    }

    pub(crate) const fn product_bounds(self) -> Rect {
        self.product_bounds
    }

    pub(crate) const fn inspector_bounds(self) -> Option<Rect> {
        self.inspector_bounds
    }
}

fn exact_pane(width: f32) -> SplitViewPane {
    SplitViewPane::new(width, width, width)
}

#[cfg(test)]
#[path = "root_layout_tests.rs"]
mod tests;
