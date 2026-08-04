use zui::GridLayout;
use zui::GridNode;
use zui::GridPane;
use zui::Rect;
use zui::SplitViewLayoutPriority;
use zui::SplitViewOrientation;
use zui::SplitViewPane;

/// Logical dimensions of a native presentation viewport.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogicalViewport {
    /// Width in logical UI pixels.
    pub width: f32,
    /// Height in logical UI pixels.
    pub height: f32,
}

impl LogicalViewport {
    /// Converts physical dimensions into logical UI pixels using a validated scale factor.
    pub fn from_physical(width: u32, height: u32, scale_factor: f64) -> Self {
        let scale_factor = if scale_factor.is_finite() && scale_factor > 0.0 {
            scale_factor as f32
        } else {
            1.0
        };
        Self {
            width: width as f32 / scale_factor,
            height: height as f32 / scale_factor,
        }
    }
}

/// Whether the developer inspector participates in the root split.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum InspectorPane {
    /// Keep the product as the only root leaf.
    #[default]
    Hidden,
    /// Add an inspector leaf using the available preferred width.
    Visible {
        /// Preferred inspector width in logical UI pixels.
        preferred_width: f32,
    },
}

impl InspectorPane {
    /// Creates a visible inspector request.
    pub const fn visible(preferred_width: f32) -> Self {
        Self::Visible { preferred_width }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum RootLeafId {
    Product,
    Inspector,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum RootSplitId {
    ProductAndInspector,
}

/// Top-level product and developer-tool leaf geometry for one native window frame.
///
/// The product leaf keeps its caller-provided viewport width. When the native window has gained
/// additional width, the inspector contributes a sibling Grid leaf instead of consuming space
/// inside the product leaf.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RootLayout {
    product_bounds: Rect,
    inspector_bounds: Option<Rect>,
}

impl RootLayout {
    /// Resolves product and inspector leaves from the full window, product viewport, and inspector
    /// request.
    pub fn for_viewports(
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

    /// Returns the product leaf bounds.
    pub const fn product_bounds(self) -> Rect {
        self.product_bounds
    }

    /// Returns the optional inspector leaf bounds.
    pub const fn inspector_bounds(self) -> Option<Rect> {
        self.inspector_bounds
    }
}

fn exact_pane(width: f32) -> SplitViewPane {
    SplitViewPane::new(width, width, width)
}

#[cfg(test)]
#[path = "root_tests.rs"]
mod tests;
