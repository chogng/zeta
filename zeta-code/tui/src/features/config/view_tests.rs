use super::config_view;
use crate::components::selection::SelectionViewState;
use crate::features::config::ConfigSelectionAction;
use crate::features::config::TerminalSettings;
use crate::test_support::empty_config_snapshot;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

#[test]
fn config_pane_organizes_the_snapshot_into_searchable_tabs() {
    let mut config = empty_config_snapshot();
    config.revision = 4;
    config.generation = 5;
    let view = config_view(&config, TerminalSettings::default(), 7);
    let mut state = SelectionViewState::new(view.model.into_body());

    assert_eq!(state.title(), "Config");
    assert!(state.search().is_some());
    assert_eq!(
        state
            .tabs()
            .iter()
            .map(|tab| tab.label())
            .collect::<Vec<_>>(),
        vec![
            "Overview",
            "Enhanced terminal",
            "Providers",
            "Language servers",
        ]
    );
    assert_eq!(state.visible_items()[0].label(), "Revision");
    assert_eq!(state.visible_items()[0].description(), Some("4"));

    let _ = state.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    let mouse = &state.visible_items()[0];
    assert_eq!(mouse.label(), "Mouse interactions");
    assert_eq!(
        mouse.description(),
        Some("Clicks and hover in interactive panes true")
    );
    assert!(matches!(
        view.actions.get(mouse.id().unwrap()).unwrap(),
        ConfigSelectionAction::SetMouseInteractions(edit)
            if edit.terminal.expected_revision == 7 && !edit.terminal.mouse_interactions
    ));
}
