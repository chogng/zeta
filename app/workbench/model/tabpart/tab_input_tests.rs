//! Workbench tab input metadata tests.

use std::path::Path;
use std::path::PathBuf;

use super::TabInput;
use super::TabInputMetadata;
use zeta_protocol::SessionId;

#[test]
fn dir_roots_preserve_primary_order_and_remove_duplicates() {
    let input = TabInput::session(
        SessionId::new("session-1").unwrap(),
        TabInputMetadata::new("Session", "zeta").with_dirs([
            PathBuf::from("/dir/zeta"),
            PathBuf::from("/dir/shared"),
            PathBuf::from("/dir/zeta"),
        ]),
    );

    assert_eq!(input.first_dir(), Some(Path::new("/dir/zeta")));
    assert_eq!(
        input.dirs(),
        [PathBuf::from("/dir/zeta"), PathBuf::from("/dir/shared"),]
    );
}
