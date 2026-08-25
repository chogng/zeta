use super::{AgentSidebarState, DEFAULT_WIDTH, MAXIMUM_WIDTH, MINIMUM_MAIN_WIDTH, MINIMUM_WIDTH};
use zeta_ui::{
    Rect, SplitViewLayout, SplitViewLayoutPriority, SplitViewOrientation, SplitViewPane,
};

#[test]
fn sidebar_is_collapsed_by_default_and_toggles_visibility() {
    let mut sidebar = AgentSidebarState::default();

    assert!(!sidebar.is_expanded());
    assert!(!sidebar.layout_spec().is_visible_for(1_000.0));
    sidebar.toggle();
    assert!(sidebar.is_expanded());
    assert!(sidebar.layout_spec().is_visible_for(1_000.0));
    sidebar.toggle();
    assert!(!sidebar.is_expanded());
}

#[test]
fn narrow_viewport_temporarily_hides_the_expanded_sidebar() {
    let sidebar = AgentSidebarState::expanded();

    assert_eq!(sidebar.layout_spec().preferred_width(), DEFAULT_WIDTH);
    assert!(
        !sidebar
            .layout_spec()
            .is_visible_for(MINIMUM_WIDTH + MINIMUM_MAIN_WIDTH - 1.0)
    );
    assert!(
        sidebar
            .layout_spec()
            .is_visible_for(MINIMUM_WIDTH + MINIMUM_MAIN_WIDTH)
    );
}

#[test]
fn expand_is_idempotent_and_keeps_the_sidebar_visible() {
    let mut sidebar = AgentSidebarState::default();

    sidebar.expand();
    sidebar.expand();

    assert!(sidebar.is_expanded());
}

#[test]
fn resizing_clamps_the_sidebar_and_persists_across_visibility() {
    let mut sidebar = AgentSidebarState::expanded();
    let layout = SplitViewLayout::new(
        Rect::from_xywh(0.0, 0.0, 1_000.0, 1.0),
        SplitViewOrientation::Horizontal,
        &[
            SplitViewPane::new(480.0, MINIMUM_MAIN_WIDTH, f32::INFINITY)
                .with_priority(SplitViewLayoutPriority::High),
            sidebar.layout_spec().pane_sizing(1_000.0),
        ],
    );
    let snapshot = layout
        .sash(0)
        .expect("expanded sidebar should expose a sash");

    assert!(sidebar.start_resizing(snapshot.resize_snapshot(), 480.0));
    assert!(sidebar.resize_to(760.0));
    assert!(sidebar.finish_resizing());
    assert_eq!(sidebar.layout_spec().preferred_width(), MINIMUM_WIDTH);

    sidebar.toggle();
    assert!(!sidebar.layout_spec().is_visible_for(1_000.0));
    sidebar.toggle();
    assert!(sidebar.layout_spec().is_visible_for(1_000.0));

    let layout = SplitViewLayout::new(
        Rect::from_xywh(0.0, 0.0, 1_200.0, 1.0),
        SplitViewOrientation::Horizontal,
        &[
            SplitViewPane::new(840.0, MINIMUM_MAIN_WIDTH, f32::INFINITY)
                .with_priority(SplitViewLayoutPriority::High),
            sidebar.layout_spec().pane_sizing(1_200.0),
        ],
    );
    let snapshot = layout
        .sash(0)
        .expect("expanded sidebar should expose a sash");
    assert!(sidebar.start_resizing(snapshot.resize_snapshot(), 840.0));
    assert!(sidebar.resize_to(200.0));
    assert!(sidebar.finish_resizing());
    assert_eq!(sidebar.layout_spec().preferred_width(), MAXIMUM_WIDTH);
}
