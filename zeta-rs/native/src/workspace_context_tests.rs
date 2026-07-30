use std::path::Path;

use super::{WorkspaceContext, display_working_directory};

#[test]
fn home_relative_working_directory_uses_a_compact_label() {
    assert_eq!(
        display_working_directory(
            Path::new("/Users/lance/Desktop/zeta"),
            Some(Path::new("/Users/lance")),
        ),
        "~/Desktop/zeta"
    );
    assert_eq!(
        display_working_directory(Path::new("/Users/lance"), Some(Path::new("/Users/lance"))),
        "~"
    );
}

#[test]
fn fixture_exposes_all_four_toolbar_values_without_inventing_git_state() {
    let repository = WorkspaceContext::fixture("~/Desktop/zeta", Some("main"), Some(3));
    let plain_directory = WorkspaceContext::fixture("/tmp/plain", None, None);

    assert_eq!(repository.location_label(), "Local");
    assert_eq!(repository.working_directory_label(), "~/Desktop/zeta");
    assert_eq!(repository.git_branch_label(), "main");
    assert_eq!(repository.diff_count_label(), "3");
    assert_eq!(plain_directory.git_branch_label(), "No Git");
    assert_eq!(plain_directory.diff_count_label(), "—");
}
