use super::{MINIMUM_MAIN_WIDTH, SessionSidebarState};

#[test]
fn expanded_sidebar_uses_its_default_width() {
    let sidebar = SessionSidebarState::expanded();

    assert_eq!(sidebar.visible_width(1_000.0), Some(200.0));
}

#[test]
fn resizing_clamps_the_preferred_width_and_preserves_it_across_visibility() {
    let mut sidebar = SessionSidebarState::expanded();

    assert!(sidebar.start_resizing(1_000.0, 200.0));
    assert!(sidebar.resize_to(360.0));
    assert!(sidebar.finish_resizing());
    assert_eq!(sidebar.visible_width(1_000.0), Some(360.0));

    sidebar.toggle();
    assert_eq!(sidebar.visible_width(1_000.0), None);
    sidebar.toggle();
    assert_eq!(sidebar.visible_width(1_000.0), Some(360.0));

    assert!(sidebar.start_resizing(1_000.0, 360.0));
    assert!(sidebar.resize_to(40.0));
    assert_eq!(sidebar.visible_width(1_000.0), Some(160.0));
    assert!(sidebar.resize_to(900.0));
    assert_eq!(sidebar.visible_width(1_000.0), Some(480.0));
}

#[test]
fn viewport_constraints_do_not_replace_the_preferred_width() {
    let mut sidebar = SessionSidebarState::expanded();
    assert!(sidebar.start_resizing(1_000.0, 200.0));
    assert!(sidebar.resize_to(420.0));
    assert!(sidebar.finish_resizing());

    assert_eq!(
        sidebar.visible_width(500.0),
        Some(500.0 - MINIMUM_MAIN_WIDTH)
    );
    assert_eq!(sidebar.visible_width(1_000.0), Some(420.0));
    assert_eq!(sidebar.visible_width(MINIMUM_MAIN_WIDTH + 159.0), None);

    assert!(sidebar.start_resizing(500.0, 260.0));
    assert!(!sidebar.resize_to(320.0));
    assert!(sidebar.finish_resizing());
    assert_eq!(sidebar.visible_width(1_000.0), Some(420.0));
}
