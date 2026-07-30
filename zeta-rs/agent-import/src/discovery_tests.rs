use std::fs;

use tempfile::tempdir;

use crate::{
    AgentImportDiagnosticCode, AgentImportLocation, AgentImportPlan, ExternalAgent, ImportItemKind,
    ImportReviewCategory, ImportScope,
};

#[test]
fn discovers_codex_user_items_without_authentication_state() {
    let home = tempdir().unwrap();
    fs::create_dir_all(home.path().join(".codex/agents")).unwrap();
    fs::create_dir_all(home.path().join(".codex/rules")).unwrap();
    fs::create_dir_all(home.path().join(".agents/skills")).unwrap();
    fs::write(home.path().join(".codex/config.toml"), "model = \"test\"").unwrap();
    fs::write(home.path().join(".codex/auth.json"), "secret").unwrap();

    let plan = AgentImportPlan::discover([AgentImportLocation::codex_user(home.path())]).unwrap();

    assert_eq!(plan.candidates().len(), 4);
    assert!(plan.diagnostics().is_empty());
    assert!(plan.candidates().iter().any(|candidate| {
        candidate.kind() == ImportItemKind::Skills
            && candidate.relative_path() == std::path::Path::new(".agents/skills")
    }));
    assert!(plan.candidates().iter().all(|candidate| {
        candidate.agent() == ExternalAgent::Codex
            && candidate.scope() == ImportScope::User
            && candidate.relative_path() != std::path::Path::new(".codex/auth.json")
    }));
    assert!(!format!("{plan:?}").contains(&home.path().display().to_string()));
}

#[test]
fn discovers_claude_user_items_without_profile_state() {
    let home = tempdir().unwrap();
    fs::create_dir_all(home.path().join(".claude/skills")).unwrap();
    fs::write(home.path().join(".claude/settings.json"), "{}").unwrap();
    fs::write(
        home.path().join(".claude.json"),
        "{\"oauthAccount\":\"secret\"}",
    )
    .unwrap();

    let plan = AgentImportPlan::discover([AgentImportLocation::claude_user(home.path())]).unwrap();

    assert_eq!(plan.candidates().len(), 2);
    assert!(plan.candidates().iter().all(|candidate| {
        candidate.agent() == ExternalAgent::Claude
            && candidate.relative_path() != std::path::Path::new(".claude.json")
    }));
}

#[test]
fn discovers_claude_project_items_with_review_categories() {
    let project = tempdir().unwrap();
    fs::create_dir_all(project.path().join(".claude/skills")).unwrap();
    fs::create_dir_all(project.path().join(".claude/agents")).unwrap();
    fs::write(project.path().join("CLAUDE.md"), "# Instructions").unwrap();
    fs::write(project.path().join(".claude/settings.json"), "{}").unwrap();
    fs::write(project.path().join(".mcp.json"), "{\"mcpServers\":{}}").unwrap();

    let plan =
        AgentImportPlan::discover([AgentImportLocation::claude_project(project.path())]).unwrap();

    assert_eq!(plan.candidates().len(), 5);
    assert!(plan.candidates().iter().any(|candidate| {
        candidate.kind() == ImportItemKind::Instructions
            && candidate.review() == ImportReviewCategory::Content
    }));
    assert!(plan.candidates().iter().any(|candidate| {
        candidate.kind() == ImportItemKind::Settings
            && candidate.review() == ImportReviewCategory::Configuration
    }));
    assert!(plan.candidates().iter().any(|candidate| {
        candidate.kind() == ImportItemKind::McpServers
            && candidate.review() == ImportReviewCategory::Connection
    }));
}

#[test]
fn reports_known_paths_with_the_wrong_file_type() {
    let project = tempdir().unwrap();
    fs::create_dir_all(project.path().join(".agents/skills")).unwrap();
    fs::create_dir_all(project.path().join(".codex/config.toml")).unwrap();

    let plan =
        AgentImportPlan::discover([AgentImportLocation::codex_project(project.path())]).unwrap();

    assert_eq!(plan.candidates().len(), 1);
    assert_eq!(plan.diagnostics().len(), 1);
    assert_eq!(
        plan.diagnostics()[0].code(),
        AgentImportDiagnosticCode::UnexpectedFileType
    );
    assert_eq!(
        plan.diagnostics()[0].relative_path(),
        std::path::Path::new(".codex/config.toml")
    );
}

#[cfg(unix)]
#[test]
fn rejects_known_paths_that_escape_through_an_ancestor_symlink() {
    use std::os::unix::fs::symlink;

    let project = tempdir().unwrap();
    let outside = tempdir().unwrap();
    fs::write(outside.path().join("settings.json"), "{}").unwrap();
    symlink(outside.path(), project.path().join(".claude")).unwrap();

    let plan =
        AgentImportPlan::discover([AgentImportLocation::claude_project(project.path())]).unwrap();

    assert!(plan.candidates().is_empty());
    assert_eq!(plan.diagnostics().len(), 1);
    assert_eq!(
        plan.diagnostics()[0].code(),
        AgentImportDiagnosticCode::EscapesSelectedRoot
    );
}
