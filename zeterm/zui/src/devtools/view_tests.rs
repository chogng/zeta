use super::ToolbarAction;
use super::compose;
use super::decorate_product_scene;
use super::row_index_at;
use super::toolbar_action_at;
use crate::devtools::DevToolsHandle;
use crate::devtools::InspectionSelection;
use crate::ui::Color;
use crate::ui::Element;
use crate::ui::Point;
use crate::ui::Rect;
use crate::ui::Size;
use crate::ui::UiScene;

#[test]
fn default_view_paints_title_and_selection_metadata() {
    let mut source = UiScene::new(Color::TRANSPARENT);
    source.with_element(
        Element::leaf("Button")
            .in_bounds(Rect::from_xywh(10.0, 10.0, 80.0, 30.0))
            .with_inspection_label("Run"),
        |_, _| {},
    );
    let handle = DevToolsHandle::new();
    handle.open();
    handle.set_inspection(source.inspection().clone());
    handle.toggle_picking();
    handle.set_hovered(InspectionSelection::at(
        source.inspection(),
        Point::new(20.0, 20.0),
    ));
    let scene = compose(
        Size::new(420.0, 520.0),
        handle.inspection().as_ref(),
        &handle,
    );
    let texts = scene
        .text_blocks()
        .iter()
        .map(|block| block.text())
        .collect::<Vec<_>>();
    assert!(texts.contains(&"ZUI DevTools"));
    assert!(texts.iter().any(|text| text.contains("Button")));
    assert!(texts.iter().any(|text| text.contains("size 80 × 30")));
    let icons = scene
        .icons()
        .iter()
        .map(|icon| icon.icon().id().as_str())
        .collect::<Vec<_>>();
    assert!(icons.contains(&"zui-devtools-pick"));
    assert!(icons.contains(&"zui-devtools-close"));
    assert!(icons.contains(&"zui-devtools-ancestor"));
}

#[test]
fn default_view_hit_tests_toolbar_and_ancestor_rows() {
    let bounds = Rect::from_xywh(0.0, 0.0, 420.0, 520.0);
    assert_eq!(
        toolbar_action_at(bounds, Point::new(300.0, 23.0)),
        Some(ToolbarAction::Pick)
    );
    assert_eq!(
        toolbar_action_at(bounds, Point::new(370.0, 23.0)),
        Some(ToolbarAction::Close)
    );
    assert_eq!(row_index_at(bounds, Point::new(20.0, 70.0), 2), Some(0));
    assert_eq!(row_index_at(bounds, Point::new(20.0, 170.0), 2), Some(1));
    assert_eq!(row_index_at(bounds, Point::new(20.0, 400.0), 2), None);
}

#[test]
fn hover_and_locked_selection_decorate_the_product_scene() {
    let mut source = UiScene::new(Color::TRANSPARENT);
    source.with_element(
        Element::leaf("Button").in_bounds(Rect::from_xywh(10.0, 10.0, 80.0, 30.0)),
        |_, _| {},
    );
    let selection = InspectionSelection::at(source.inspection(), Point::new(20.0, 20.0))
        .expect("point should select a node");
    let handle = DevToolsHandle::new();
    handle.open();
    handle.set_inspection(source.inspection().clone());
    handle.toggle_picking();
    handle.set_hovered(Some(selection.clone()));

    let hovered = decorate_product_scene(&source, &handle).expect("hover should decorate");
    assert!(hovered.rects().len() > source.rects().len());

    handle.set_hovered(None);
    assert!(decorate_product_scene(&source, &handle).is_none());

    handle.set_hovered(Some(selection.clone()));
    handle.select(Some(selection));
    let locked = decorate_product_scene(&source, &handle).expect("selection should decorate");
    assert!(locked.rects().len() > source.rects().len());
}
