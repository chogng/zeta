use super::InspectorPane;
use super::LogicalViewport;
use super::RootLayout;
use zui::Rect;

#[test]
fn inspector_is_a_sibling_grid_leaf_outside_the_preserved_product_viewport() {
    let layout = RootLayout::for_viewports(
        LogicalViewport {
            width: 1_360.0,
            height: 700.0,
        },
        LogicalViewport {
            width: 1_000.0,
            height: 700.0,
        },
        InspectorPane::visible(360.0),
    );

    assert_eq!(
        layout.product_bounds(),
        Rect::from_xywh(0.0, 0.0, 1_000.0, 700.0)
    );
    assert_eq!(
        layout.inspector_bounds(),
        Some(Rect::from_xywh(1_000.0, 0.0, 360.0, 700.0))
    );
}

#[test]
fn hidden_inspector_leaves_the_product_as_the_only_root_grid_leaf() {
    let layout = RootLayout::for_viewports(
        LogicalViewport {
            width: 1_360.0,
            height: 700.0,
        },
        LogicalViewport {
            width: 1_000.0,
            height: 700.0,
        },
        InspectorPane::Hidden,
    );

    assert_eq!(
        layout.product_bounds(),
        Rect::from_xywh(0.0, 0.0, 1_000.0, 700.0)
    );
    assert_eq!(layout.inspector_bounds(), None);
}
