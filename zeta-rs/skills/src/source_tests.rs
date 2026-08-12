use super::*;
use std::fs;

#[test]
fn source_root_debug_output_does_not_expose_the_host_path() {
    let root = std::env::temp_dir().join(format!("zeta-skill-source-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let source =
        SkillSourceRoot::user(SkillSourceId::new("user:skill-source:test").unwrap(), &root)
            .unwrap();

    let rendered = format!("{source:?}");

    assert!(!rendered.contains(root.to_str().unwrap()));
    assert!(rendered.contains("<private>"));
    let _ = fs::remove_dir(&root);
}

#[test]
fn workspace_source_preserves_workspace_provenance() {
    let root = std::env::temp_dir().join(format!(
        "zeta-workspace-skill-source-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    let source = SkillSourceRoot::workspace(
        SkillSourceId::new("workspace:skill-source:.zeta").unwrap(),
        &root,
    )
    .unwrap();

    assert_eq!(source.view().kind(), SkillSourceKind::Workspace);
    assert_eq!(source.view().trust(), SkillTrust::WorkspaceManaged);
    let _ = fs::remove_dir(&root);
}

#[cfg(unix)]
#[test]
fn source_root_rejects_symbolic_links() {
    use std::os::unix::fs::symlink;

    let base = std::env::temp_dir().join(format!("zeta-skill-link-{}", std::process::id()));
    let target = base.join("target");
    let link = base.join("link");
    fs::create_dir_all(&target).unwrap();
    symlink(&target, &link).unwrap();

    let error = SkillSourceRoot::user(SkillSourceId::new("user:skill-source:test").unwrap(), &link)
        .unwrap_err();

    assert_eq!(error.kind(), SkillErrorKind::SourceUnavailable);
    let _ = fs::remove_dir_all(&base);
}
