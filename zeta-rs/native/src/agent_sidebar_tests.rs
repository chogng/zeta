use super::{AgentSidebarState, DEFAULT_WIDTH, MINIMUM_MAIN_WIDTH};

#[test]
fn sidebar_is_collapsed_by_default_and_toggles_visibility() {
    let mut sidebar = AgentSidebarState::default();

    assert!(!sidebar.is_expanded());
    assert!(!sidebar.is_visible_for(1_000.0));
    sidebar.toggle();
    assert!(sidebar.is_expanded());
    assert!(sidebar.is_visible_for(1_000.0));
    sidebar.toggle();
    assert!(!sidebar.is_expanded());
}

#[test]
fn narrow_viewport_temporarily_hides_the_expanded_sidebar() {
    let sidebar = AgentSidebarState::expanded();

    assert_eq!(sidebar.preferred_width(), DEFAULT_WIDTH);
    assert!(!sidebar.is_visible_for(DEFAULT_WIDTH + MINIMUM_MAIN_WIDTH - 1.0));
    assert!(sidebar.is_visible_for(DEFAULT_WIDTH + MINIMUM_MAIN_WIDTH));
}
