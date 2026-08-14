use zeta_remote::RemoteWorkspacePath;
use zeta_remote::SshHost;
use zeta_remote::SshTarget;
use zeta_remote_connections::RemoteConnectionEntry;
use zeta_remote_connections::RemoteConnectionName;
use zeta_ui::CaretVisibility;
use zeta_ui::Rect;
use zeta_ui::TextInputCommand;
use zeta_ui::TextInputLayoutEngine;
use zui::AccessibilityRole;
use zui::InteractionFrame;
use zui::UiDispatch;
use zui::UiFrame;

use super::RemoteConnectionPicker;
use super::RemoteConnectionPickerAction;
use super::RemoteConnectionPickerState;
use crate::shell_interaction::COMPOSER;
use crate::shell_style::SHELL_PALETTE;

#[test]
fn picker_sorts_filters_and_activates_canonical_connection_names() {
    let mut state = RemoteConnectionPickerState::default();
    state.open(
        Rect::from_xywh(40.0, 640.0, 80.0, 24.0),
        vec![
            connection("zulu", "zulu.example", "/srv/backend"),
            connection("alpha", "build.example", "/work/frontend"),
        ],
        false,
        Some(COMPOSER),
    );

    assert_eq!(
        state.activate(0),
        Some(RemoteConnectionPickerAction::Manage)
    );
    assert!(state.items()[1].label.starts_with("alpha · build.example"));
    assert_eq!(
        state.activate(1),
        Some(RemoteConnectionPickerAction::Connect(
            RemoteConnectionName::parse("alpha").unwrap()
        ))
    );
    state.apply_search(TextInputCommand::Insert("BACK".into()));
    assert_eq!(state.items().len(), 1);
    assert!(state.items()[0].label.contains("/srv/backend"));
    assert_eq!(
        state.activate(0),
        Some(RemoteConnectionPickerAction::Connect(
            RemoteConnectionName::parse("zulu").unwrap()
        ))
    );
    assert_eq!(state.dismiss(), Some(COMPOSER));
}

#[test]
fn empty_catalogs_offer_native_management_and_unmatched_searches_are_passive() {
    let mut state = RemoteConnectionPickerState::default();
    state.open(
        Rect::from_xywh(40.0, 640.0, 80.0, 24.0),
        Vec::new(),
        false,
        None,
    );
    assert_eq!(state.items()[0].label, "Manage Remote connections…");
    assert_eq!(
        state.activate(0),
        Some(RemoteConnectionPickerAction::Manage)
    );
    assert!(state.first_action_id().is_some());

    state.open(
        Rect::from_xywh(40.0, 640.0, 80.0, 24.0),
        vec![connection("build", "build.example", "/srv/project")],
        false,
        None,
    );
    state.apply_search(TextInputCommand::Insert("missing".into()));
    assert_eq!(state.items()[0].label, "No matching Remote connections");
    assert!(state.activate(0).is_none());
}

#[test]
fn picker_is_modal_accessible_and_anchored_above_the_location_button() {
    let anchor = Rect::from_xywh(40.0, 640.0, 80.0, 24.0);
    let mut state = RemoteConnectionPickerState::default();
    state.open(
        anchor,
        vec![connection("build", "build.example", "/srv/project")],
        false,
        None,
    );
    let dispatch = UiDispatch::default();
    let mut text_layout = TextInputLayoutEngine::new();
    let picker = RemoteConnectionPicker::new(
        Rect::from_xywh(0.0, 0.0, 1_000.0, 700.0),
        &state,
        CaretVisibility::Visible,
        SHELL_PALETTE,
        &mut text_layout,
        &dispatch,
    )
    .unwrap();
    let mut frame = UiFrame::<InteractionFrame>::new(SHELL_PALETTE.background);
    frame.draw_component(&picker);

    assert!(picker.bounds().bottom() <= anchor.origin.y);
    let nodes = frame.interaction().accessibility_nodes(&dispatch);
    assert_eq!(nodes[0].role, AccessibilityRole::Menu);
    assert_eq!(nodes[1].role, AccessibilityRole::TextInput);
    assert_eq!(nodes[2].role, AccessibilityRole::MenuItem);
    assert_eq!(nodes[3].role, AccessibilityRole::MenuItem);
}

#[test]
fn remote_windows_offer_native_tunnel_management() {
    let mut state = RemoteConnectionPickerState::default();
    state.open(
        Rect::from_xywh(40.0, 640.0, 80.0, 24.0),
        Vec::new(),
        true,
        None,
    );

    assert_eq!(state.items()[1].label, "Manage Remote tunnels…");
    assert_eq!(
        state.activate(1),
        Some(RemoteConnectionPickerAction::ManageTunnels)
    );
}

fn connection(name: &str, host: &str, workspace: &str) -> RemoteConnectionEntry {
    RemoteConnectionEntry::new(
        RemoteConnectionName::parse(name).unwrap(),
        SshTarget::new(
            SshHost::parse(host).unwrap(),
            RemoteWorkspacePath::parse(workspace).unwrap(),
        ),
    )
}
