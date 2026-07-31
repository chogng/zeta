use super::workspace_title;
use std::path::Path;

#[test]
fn workspace_title_uses_the_last_path_component() {
    assert_eq!(workspace_title(Path::new("/work/zeta")), "zeta");
}

#[test]
fn workspace_title_has_a_stable_root_fallback() {
    assert_eq!(workspace_title(Path::new("/")), "Agent Session");
}
