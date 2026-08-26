//! Pane group tree and split-layout contract tests.

use super::{PaneGroup, PaneId, PaneSplitDirection};
use zeta_ui::Rect;

#[test]
fn a_group_starts_with_one_active_leaf() {
    let group = PaneGroup::new();

    assert_eq!(group.leaf_ids(), [PaneId::ROOT]);
    assert_eq!(group.active_pane(), PaneId::ROOT);
}

#[test]
fn splitting_active_pane_creates_a_new_active_sibling() {
    let mut group = PaneGroup::new();

    let second = group.split_active(PaneSplitDirection::Horizontal);

    assert_eq!(group.leaf_ids(), [PaneId::ROOT, second]);
    assert_eq!(group.active_pane(), second);
}

#[test]
fn nested_split_and_close_collapse_the_owning_split() {
    let mut group = PaneGroup::new();
    let second = group.split_active(PaneSplitDirection::Horizontal);
    let third = group.split_active(PaneSplitDirection::Vertical);

    assert_eq!(group.leaf_ids(), [PaneId::ROOT, second, third]);
    assert_eq!(group.close_active(), Some(third));
    assert_eq!(group.leaf_ids(), [PaneId::ROOT, second]);
    assert_eq!(group.active_pane(), second);
}

#[test]
fn the_last_pane_cannot_be_closed() {
    let mut group = PaneGroup::new();

    assert_eq!(group.close_active(), None);
    assert_eq!(group.leaf_ids(), [PaneId::ROOT]);
}

#[test]
fn focus_wraps_over_visual_leaf_order() {
    let mut group = PaneGroup::new();
    let second = group.split_active(PaneSplitDirection::Horizontal);
    let third = group.split_active(PaneSplitDirection::Vertical);

    assert_eq!(group.focus_previous(), second);
    assert_eq!(group.focus_previous(), PaneId::ROOT);
    assert_eq!(group.focus_previous(), third);
    assert_eq!(group.focus_next(), PaneId::ROOT);
}

#[test]
fn layout_exposes_one_leaf_per_pane_and_sash_per_split() {
    let mut group = PaneGroup::new();
    group.split_active(PaneSplitDirection::Horizontal);
    group.split_active(PaneSplitDirection::Vertical);

    let layout = group.layout(Rect::from_xywh(0.0, 0.0, 800.0, 600.0));

    assert_eq!(layout.leaves().len(), 3);
    assert_eq!(layout.sashes().len(), 2);
}

#[test]
fn resizing_a_split_persists_its_ratio_in_the_next_layout() {
    let mut group = PaneGroup::new();
    group.split_active(PaneSplitDirection::Horizontal);
    let initial = group.layout(Rect::from_xywh(0.0, 0.0, 800.0, 600.0));
    let sash = initial.sashes()[0];
    let resize = sash.resize_snapshot().resize(120.0);

    assert!(group.resize_split(sash.split_id(), resize));

    let resized = group.layout(Rect::from_xywh(0.0, 0.0, 800.0, 600.0));
    assert!(
        resized
            .leaf(crate::pane_group::PaneId::ROOT)
            .unwrap()
            .bounds()
            .size
            .width
            > 400.0
    );
}
