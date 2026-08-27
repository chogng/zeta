use crate::TabInput;
use crate::TabInputMetadata;
use crate::TabStatus;
use zeta_protocol::Session;

/// Builds the Workbench tab description owned by one Session.
pub fn session_tab_input(session: &Session, workspace_label: &str) -> TabInput {
    let workspace = session
        .workspace
        .as_ref()
        .and_then(|binding| binding.root.file_name())
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or(workspace_label);
    let mut metadata =
        TabInputMetadata::new(&session.title, workspace).with_status(TabStatus::idle("Ready"));
    if let Some(workspace_root) = session
        .workspace
        .as_ref()
        .map(|binding| binding.root.clone())
    {
        metadata = metadata.with_workspace_root(workspace_root);
    }
    TabInput::session(session.session_id.clone(), metadata)
}
