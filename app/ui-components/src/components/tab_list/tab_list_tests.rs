use super::{
    Tab, TabBackgrounds, TabList, TabListOrientation, TabListStyle, TabSelection, TabState,
    TabStyle,
};
use crate::{Color, CornerRadii, Rect, Size, UiScene};

#[test]
fn vertical_tab_list_owns_tab_size_gap_and_surface_presentation() {
    let highlight = Color::rgb(235, 235, 237);
    let backgrounds = TabBackgrounds::new(Color::TRANSPARENT)
        .with_hovered(highlight)
        .with_focused(highlight)
        .with_pressed(highlight);
    let style = TabListStyle::new(
        TabStyle::new(backgrounds)
            .with_selected_backgrounds(TabBackgrounds::new(highlight))
            .with_corner_radii(CornerRadii::uniform(4.0)),
        Size::new(180.0, 52.0),
    )
    .with_gap(6.0);
    let list = TabList::new(
        Rect::from_xywh(10.0, 20.0, 180.0, 200.0),
        TabListOrientation::Vertical,
        vec![
            Tab::new(TabState::Resting).with_selection(TabSelection::Selected),
            Tab::new(TabState::Hovered),
        ],
        style,
    );
    let mut scene = UiScene::new(Color::WHITE);

    scene.draw_component(&list);

    let first = list.tab_bounds(0).unwrap();
    let second = list.tab_bounds(1).unwrap();
    assert_eq!(first, Rect::from_xywh(10.0, 20.0, 180.0, 52.0));
    assert_eq!(second.origin.y - first.bottom(), 6.0);
    assert_eq!(scene.rects()[0].fill(), highlight);
    assert_eq!(scene.rects()[1].fill(), highlight);
    assert_eq!(scene.rects()[0].corner_radii(), CornerRadii::uniform(4.0));
    let node = scene
        .inspection()
        .nodes()
        .iter()
        .find(|node| node.name() == "TabList")
        .expect("TabList inspection node");
    assert_eq!(node.gap(), Some(6.0));
    assert_eq!(
        node.gap_regions(),
        &[Rect::from_xywh(10.0, 72.0, 180.0, 6.0)]
    );
    assert_eq!(
        scene
            .inspection()
            .nodes()
            .iter()
            .map(|node| node.name())
            .collect::<Vec<_>>(),
        ["TabList"]
    );
}

#[test]
fn horizontal_tab_list_clips_tabs_to_its_available_width() {
    let style = TabListStyle::new(
        TabStyle::new(TabBackgrounds::new(Color::TRANSPARENT)),
        Size::new(100.0, 32.0),
    )
    .with_gap(6.0);
    let list = TabList::new(
        Rect::from_xywh(0.0, 0.0, 180.0, 32.0),
        TabListOrientation::Horizontal,
        vec![Tab::new(TabState::Resting), Tab::new(TabState::Resting)],
        style,
    );

    assert_eq!(
        list.tab_bounds(0),
        Some(Rect::from_xywh(0.0, 0.0, 100.0, 32.0))
    );
    assert_eq!(
        list.tab_bounds(1),
        Some(Rect::from_xywh(106.0, 0.0, 74.0, 32.0))
    );
    assert_eq!(list.tab_bounds(2), None);
}
