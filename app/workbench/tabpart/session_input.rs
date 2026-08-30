use crate::TabInput;
use crate::TabInputMetadata;
use crate::TabStatus;
use zeta_protocol::Session;

/// Builds the Workbench tab description owned by one Session.
pub fn session_tab_input(session: &Session, cwd_label: &str) -> TabInput {
    let metadata =
        TabInputMetadata::new(&session.title, cwd_label).with_status(TabStatus::idle("Ready"));
    TabInput::session(session.session_id.clone(), metadata)
}
