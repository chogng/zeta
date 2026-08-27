use super::config_view;
use crate::components::selection::SelectionViewState;
use crate::test_support::empty_config_snapshot;

#[test]
fn config_pane_organizes_the_snapshot_into_searchable_tabs() {
    let mut config = empty_config_snapshot();
    config.revision = 4;
    config.generation = 5;
    let state = SelectionViewState::new(config_view(&config).into_body());

    assert_eq!(state.title(), "Config");
    assert!(state.search().is_some());
    assert_eq!(
        state
            .tabs()
            .iter()
            .map(|tab| tab.label())
            .collect::<Vec<_>>(),
        vec!["Overview", "Providers", "Language servers",]
    );
    assert_eq!(state.visible_items()[0].label(), "Revision");
    assert_eq!(state.visible_items()[0].description(), Some("4"));
}
