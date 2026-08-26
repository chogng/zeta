//! Sessions sidebar sizing and visibility tests.

use std::time::Instant;

use super::{MINIMUM_MAIN_WIDTH, SessionSidebarState};
use zeta_ui::Point;
use zeta_ui::SashPointerPresence;

#[test]
fn sidebar_is_expanded_by_default_and_can_still_be_collapsed() {
    let mut sidebar = SessionSidebarState::default();

    assert!(sidebar.is_expanded());
    assert_eq!(sidebar.visible_width(1_000.0), Some(200.0));

    sidebar.toggle();
    assert!(!sidebar.is_expanded());
    assert_eq!(sidebar.visible_width(1_000.0), None);

    sidebar.toggle();
    assert!(sidebar.is_expanded());
}

#[test]
fn expanded_sidebar_uses_its_default_width() {
    let sidebar = SessionSidebarState::expanded();

    assert_eq!(sidebar.visible_width(1_000.0), Some(200.0));
}

#[test]
fn resizing_clamps_the_preferred_width_and_preserves_it_across_visibility() {
    let now = Instant::now();
    let mut sidebar = SessionSidebarState::expanded();

    assert!(sidebar.start_resizing(1_000.0, Point::new(200.0, 0.0), now));
    assert!(sidebar.resize_to(Point::new(360.0, 0.0)));
    assert!(sidebar.finish_resizing(SashPointerPresence::Outside, now));
    assert_eq!(sidebar.visible_width(1_000.0), Some(360.0));

    sidebar.toggle();
    assert_eq!(sidebar.visible_width(1_000.0), None);
    sidebar.toggle();
    assert_eq!(sidebar.visible_width(1_000.0), Some(360.0));

    assert!(sidebar.start_resizing(1_000.0, Point::new(360.0, 0.0), now));
    assert!(sidebar.resize_to(Point::new(40.0, 0.0)));
    assert_eq!(sidebar.visible_width(1_000.0), Some(160.0));
    assert!(sidebar.resize_to(Point::new(900.0, 0.0)));
    assert_eq!(sidebar.visible_width(1_000.0), Some(480.0));
}

#[test]
fn viewport_constraints_do_not_replace_the_preferred_width() {
    let now = Instant::now();
    let mut sidebar = SessionSidebarState::expanded();
    assert!(sidebar.start_resizing(1_000.0, Point::new(200.0, 0.0), now));
    assert!(sidebar.resize_to(Point::new(420.0, 0.0)));
    assert!(sidebar.finish_resizing(SashPointerPresence::Outside, now));

    assert_eq!(
        sidebar.visible_width(500.0),
        Some(500.0 - MINIMUM_MAIN_WIDTH)
    );
    assert_eq!(sidebar.visible_width(1_000.0), Some(420.0));
    assert_eq!(sidebar.visible_width(MINIMUM_MAIN_WIDTH + 159.0), None);

    assert!(sidebar.start_resizing(500.0, Point::new(260.0, 0.0), now));
    assert!(!sidebar.resize_to(Point::new(320.0, 0.0)));
    assert!(sidebar.finish_resizing(SashPointerPresence::Outside, now));
    assert_eq!(sidebar.visible_width(1_000.0), Some(420.0));
}
