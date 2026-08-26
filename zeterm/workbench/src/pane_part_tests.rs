//! Recursive split-layout contract tests.

use super::{PaneGroupId, PaneInput, PanePart, PaneSplitDirection};
use zeta_ui::Rect;

#[test]
fn a_layout_starts_with_one_active_group() {
    let layout = PanePart::new();

    assert_eq!(layout.group_ids(), [PaneGroupId::ROOT]);
    assert_eq!(layout.active_group(), PaneGroupId::ROOT);
}

#[test]
fn splitting_active_group_creates_a_new_active_sibling() {
    let mut layout = PanePart::new();

    let second = layout.split_active(PaneSplitDirection::Horizontal);

    assert_eq!(layout.group_ids(), [PaneGroupId::ROOT, second]);
    assert_eq!(layout.active_group(), second);
}

#[test]
fn nested_split_and_close_collapse_the_owning_split() {
    let mut layout = PanePart::new();
    let second = layout.split_active(PaneSplitDirection::Horizontal);
    let third = layout.split_active(PaneSplitDirection::Vertical);

    assert_eq!(layout.group_ids(), [PaneGroupId::ROOT, second, third]);
    assert_eq!(layout.close_active().map(|(id, _)| id), Some(third));
    assert_eq!(layout.group_ids(), [PaneGroupId::ROOT, second]);
    assert_eq!(layout.active_group(), second);
}

#[test]
fn the_last_group_cannot_be_closed() {
    let mut layout = PanePart::new();

    assert!(layout.close_active().is_none());
    assert_eq!(layout.group_ids(), [PaneGroupId::ROOT]);
}

#[test]
fn focus_wraps_over_visual_group_order() {
    let mut layout = PanePart::new();
    let second = layout.split_active(PaneSplitDirection::Horizontal);
    let third = layout.split_active(PaneSplitDirection::Vertical);

    assert_eq!(layout.focus_previous(), second);
    assert_eq!(layout.focus_previous(), PaneGroupId::ROOT);
    assert_eq!(layout.focus_previous(), third);
    assert_eq!(layout.focus_next(), PaneGroupId::ROOT);
}

#[test]
fn resizing_a_split_persists_its_ratio_in_the_next_layout() {
    let mut layout = PanePart::new();
    layout.split_active(PaneSplitDirection::Horizontal);
    let initial = layout.layout(Rect::from_xywh(0.0, 0.0, 800.0, 600.0));
    let sash = initial.sashes()[0];
    let resize = sash.resize_snapshot().resize(120.0);

    assert!(layout.resize_split(sash.split_id(), resize));

    let resized = layout.layout(Rect::from_xywh(0.0, 0.0, 800.0, 600.0));
    assert!(resized.leaf(PaneGroupId::ROOT).unwrap().bounds().size.width > 400.0);
}

#[test]
fn layout_exposes_one_leaf_per_group_and_sash_per_split() {
    let mut layout = PanePart::with_input(PaneInput::settings());
    let second = layout.split_active(PaneSplitDirection::Horizontal);
    let _third = layout.split_active(PaneSplitDirection::Vertical);
    let projected = layout.layout(Rect::from_xywh(0.0, 0.0, 900.0, 600.0));

    assert_eq!(projected.leaves().len(), 3);
    assert_eq!(projected.sashes().len(), 2);
    assert!(projected.leaf(second).is_some());
}
