//! Inspector visibility, sizing, and resize-state contract tests.

use std::time::Instant;

use super::{DEFAULT_WIDTH, InspectorPartState, MAXIMUM_WIDTH, MINIMUM_MAIN_WIDTH, MINIMUM_WIDTH};
use zeta_ui::{
    Point, Rect, SashPointerPresence, SplitViewLayout, SplitViewLayoutPriority,
    SplitViewOrientation, SplitViewPane,
};

#[test]
fn inspector_is_collapsed_by_default_and_toggles_visibility() {
    let mut inspector = InspectorPartState::default();

    assert!(!inspector.is_expanded());
    assert!(!inspector.layout_spec().is_visible_for(1_000.0));
    inspector.toggle();
    assert!(inspector.is_expanded());
    assert!(inspector.layout_spec().is_visible_for(1_000.0));
    inspector.toggle();
    assert!(!inspector.is_expanded());
}

#[test]
fn narrow_viewport_temporarily_hides_the_expanded_inspector() {
    let inspector = InspectorPartState::expanded();

    assert_eq!(inspector.layout_spec().preferred_width(), DEFAULT_WIDTH);
    assert!(
        !inspector
            .layout_spec()
            .is_visible_for(MINIMUM_WIDTH + MINIMUM_MAIN_WIDTH - 1.0)
    );
    assert!(
        inspector
            .layout_spec()
            .is_visible_for(MINIMUM_WIDTH + MINIMUM_MAIN_WIDTH)
    );
}

#[test]
fn expand_is_idempotent_and_keeps_the_inspector_visible() {
    let mut inspector = InspectorPartState::default();

    inspector.expand();
    inspector.expand();

    assert!(inspector.is_expanded());
}

#[test]
fn resizing_clamps_the_inspector_and_persists_across_visibility() {
    let now = Instant::now();
    let mut inspector = InspectorPartState::expanded();
    let layout = SplitViewLayout::new(
        Rect::from_xywh(0.0, 0.0, 1_000.0, 1.0),
        SplitViewOrientation::Horizontal,
        &[
            SplitViewPane::new(480.0, MINIMUM_MAIN_WIDTH, f32::INFINITY)
                .with_priority(SplitViewLayoutPriority::High),
            inspector.layout_spec().pane_sizing(1_000.0),
        ],
    );
    let snapshot = layout
        .sash(0)
        .expect("expanded inspector should expose a sash");

    assert!(inspector.start_resizing(snapshot.resize_snapshot(), Point::new(480.0, 0.0), now,));
    assert!(inspector.resize_to(Point::new(760.0, 0.0)));
    assert!(inspector.finish_resizing(SashPointerPresence::Outside, now));
    assert_eq!(inspector.layout_spec().preferred_width(), MINIMUM_WIDTH);

    inspector.toggle();
    assert!(!inspector.layout_spec().is_visible_for(1_000.0));
    inspector.toggle();
    assert!(inspector.layout_spec().is_visible_for(1_000.0));

    let layout = SplitViewLayout::new(
        Rect::from_xywh(0.0, 0.0, 1_200.0, 1.0),
        SplitViewOrientation::Horizontal,
        &[
            SplitViewPane::new(840.0, MINIMUM_MAIN_WIDTH, f32::INFINITY)
                .with_priority(SplitViewLayoutPriority::High),
            inspector.layout_spec().pane_sizing(1_200.0),
        ],
    );
    let snapshot = layout
        .sash(0)
        .expect("expanded inspector should expose a sash");
    assert!(inspector.start_resizing(snapshot.resize_snapshot(), Point::new(840.0, 0.0), now,));
    assert!(inspector.resize_to(Point::new(200.0, 0.0)));
    assert!(inspector.finish_resizing(SashPointerPresence::Outside, now));
    assert_eq!(inspector.layout_spec().preferred_width(), MAXIMUM_WIDTH);
}
