use zeta_remote::RemoteWorkspacePath;
use zeta_remote::SshHost;
use zeta_remote::SshTarget;
use zeta_remote_connections::RemoteConnectionEntry;
use zeta_remote_connections::RemoteConnectionName;
use zeta_ui::CaretVisibility;
use zeta_ui::Rect;
use zeta_ui::TextInputLayoutEngine;
use zui::AccessibilityRole;
use zui::InteractionFrame;
use zui::UiDispatch;
use zui::UiFrame;

use super::RemoteConnectionManager;
use crate::remote_connection_manager::REMOTE_CONNECTION_MANAGER_CONNECT;
use crate::remote_connection_manager::REMOTE_CONNECTION_MANAGER_LIST;
use crate::remote_connection_manager::REMOTE_CONNECTION_MANAGER_NAME;
use crate::remote_connection_manager::REMOTE_CONNECTION_MANAGER_SAVE;
use crate::remote_connection_manager::REMOTE_CONNECTION_MANAGER_STATUS;
use crate::remote_connection_manager::RemoteConnectionManagerState;
use crate::remote_connection_manager::remote_connection_manager_item_id;
use crate::shell_style::SHELL_PALETTE;

#[test]
fn manager_is_modal_and_projects_form_list_and_actions_accessibly() {
    let mut state = RemoteConnectionManagerState::default();
    state.open(
        vec![connection("build", "build.example", "/srv/project")],
        None,
    );
    state.launch_started(RemoteConnectionName::parse("build").unwrap());
    let dispatch = UiDispatch::default();
    let mut text_layout = TextInputLayoutEngine::new();
    let manager = RemoteConnectionManager::new(
        Rect::from_xywh(0.0, 0.0, 1_000.0, 700.0),
        &state,
        CaretVisibility::Visible,
        SHELL_PALETTE,
        &mut text_layout,
        &dispatch,
    )
    .unwrap();
    let mut frame = UiFrame::<InteractionFrame>::new(SHELL_PALETTE.background);
    frame.draw_component(&manager);

    let nodes = frame.interaction().accessibility_nodes(&dispatch);
    assert!(nodes.iter().any(|node| {
        node.id == REMOTE_CONNECTION_MANAGER_NAME && node.role == AccessibilityRole::TextInput
    }));
    assert!(nodes.iter().any(|node| {
        node.id == REMOTE_CONNECTION_MANAGER_LIST && node.role == AccessibilityRole::List
    }));
    assert!(nodes.iter().any(|node| {
        node.id == remote_connection_manager_item_id(0) && node.role == AccessibilityRole::ListItem
    }));
    assert!(nodes.iter().any(|node| {
        node.id == REMOTE_CONNECTION_MANAGER_SAVE && node.role == AccessibilityRole::Button
    }));
    assert!(nodes.iter().any(|node| {
        node.id == REMOTE_CONNECTION_MANAGER_CONNECT && node.role == AccessibilityRole::Button
    }));
    assert!(nodes.iter().any(|node| {
        node.id == REMOTE_CONNECTION_MANAGER_STATUS && node.label.contains("Starting Remote window")
    }));
    assert!(manager.panel_bounds().size.width <= 720.0);
    assert_eq!(manager.list_scroll_metrics().content().height, 34.0);
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
