use super::*;
use crate::HostEnvironment;
use crate::WorkspaceRoots;

#[test]
fn render_is_deterministic_and_escapes_host_values_and_paths() {
    let workspace = absolute_path("workspace&main");
    let additional = absolute_path("other>root");
    let snapshot = AgentEnvironmentSnapshot::new(
        HostEnvironment::new(
            workspace,
            "darwin".into(),
            "Darwin <version>".into(),
            "/bin/zsh".into(),
            "2026-08-27".into(),
        )
        .unwrap(),
        RepositoryEnvironment::git(
            Some("feature/<agent>".into()),
            Some("main".into()),
            " M src/lib.rs".into(),
            "abc123 'message'".into(),
        )
        .unwrap(),
        WorkspaceRoots::new(absolute_path("workspace&main"), [additional]).unwrap(),
    );

    let rendered = snapshot.render();

    assert!(rendered.contains("workspace&amp;main</cwd>"));
    assert!(rendered.contains("<os_version>Darwin &lt;version&gt;</os_version>"));
    assert!(rendered.contains("<git_branch>feature/&lt;agent&gt;</git_branch>"));
    assert!(
        rendered.contains("<git_recent_commits>abc123 &apos;message&apos;</git_recent_commits>")
    );
    assert!(rendered.contains("other&gt;root</root>"));
    assert_eq!(rendered, snapshot.render());
}

#[test]
fn missing_repository_is_rendered_explicitly() {
    let workspace = absolute_path("workspace");
    let snapshot = AgentEnvironmentSnapshot::new(
        HostEnvironment::new(
            workspace.clone(),
            "linux".into(),
            "Linux".into(),
            "/bin/bash".into(),
            "2026-08-27".into(),
        )
        .unwrap(),
        RepositoryEnvironment::NotDetected,
        WorkspaceRoots::new(workspace, std::iter::empty()).unwrap(),
    );

    let rendered = snapshot.render();

    assert!(rendered.contains("<is_git_repo>false</is_git_repo>"));
    assert!(rendered.contains("<git_branch>(none)</git_branch>"));
}

fn absolute_path(name: &str) -> std::path::PathBuf {
    std::env::current_dir().unwrap().join(name)
}
