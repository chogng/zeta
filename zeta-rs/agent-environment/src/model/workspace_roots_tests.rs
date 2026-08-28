use super::*;

#[test]
fn primary_root_stays_first_while_additional_roots_are_sorted_and_deduplicated() {
    let workspace = absolute_path("workspace");
    let alpha = absolute_path("alpha");
    let zeta = absolute_path("zeta");
    let roots = WorkspaceRoots::new(
        workspace.clone(),
        [zeta.clone(), alpha.clone(), zeta.clone(), workspace.clone()],
    )
    .unwrap();

    assert_eq!(roots.as_slice(), &[workspace, alpha, zeta]);
}

#[test]
fn relative_roots_are_rejected_at_the_value_boundary() {
    assert!(matches!(
        WorkspaceRoots::new("workspace".into(), std::iter::empty()),
        Err(AgentEnvironmentError::PathMustBeAbsolute {
            field: "primary workspace root",
            ..
        })
    ));
    assert!(matches!(
        WorkspaceRoots::new(absolute_path("workspace"), ["relative".into()]),
        Err(AgentEnvironmentError::PathMustBeAbsolute {
            field: "additional workspace root",
            ..
        })
    ));
}

fn absolute_path(name: &str) -> std::path::PathBuf {
    std::env::current_dir().unwrap().join(name)
}
