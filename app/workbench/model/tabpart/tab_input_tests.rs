//! Workbench tab input metadata tests.

use std::path::Path;
use std::path::PathBuf;

use super::TabInput;
use super::TabInputMetadata;
use zeta_protocol::SessionId;

#[test]
fn workspace_roots_preserve_primary_order_and_remove_duplicates() {
    let input = TabInput::session(
        SessionId::new("session-1").unwrap(),
        TabInputMetadata::new("Session", "zeta").with_workspace_roots([
            PathBuf::from("/workspace/zeta"),
            PathBuf::from("/workspace/shared"),
            PathBuf::from("/workspace/zeta"),
        ]),
    );

    assert_eq!(input.workspace_root(), Some(Path::new("/workspace/zeta")));
    assert_eq!(
        input.workspace_roots(),
        [
            PathBuf::from("/workspace/zeta"),
            PathBuf::from("/workspace/shared"),
        ]
    );
}
