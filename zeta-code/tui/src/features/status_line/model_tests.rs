use super::*;
use std::path::Path;
use zeta_app_server_protocol::protocol::config::ModelRefDto;
use zeta_app_server_protocol::protocol::git::GitHeadDto;
use zeta_app_server_protocol::protocol::git::GitStatusResult;
use zeta_protocol::StreamInstanceId;

#[test]
fn wide_status_line_prefers_full_model_and_workspace_values() {
    let mut status_line = StatusLineModel::for_workspace(Path::new("/work/zeta"));
    status_line.apply_preferred_model(Some(&model("anthropic", "claude-sonnet")));

    assert_eq!(
        status_line.text_for_width(80),
        "anthropic/claude-sonnet · /work/zeta"
    );
}

#[test]
fn narrow_status_line_uses_compact_values_before_hiding_workspace() {
    let mut status_line = StatusLineModel::for_workspace(Path::new("/work/zeta"));
    status_line.apply_preferred_model(Some(&model("anthropic", "claude-sonnet")));

    assert_eq!(status_line.text_for_width(20), "claude-sonnet · zeta");
    assert_eq!(status_line.text_for_width(13), "claude-sonnet");
}

#[test]
fn status_line_without_a_configured_model_shows_the_workspace() {
    let status_line = StatusLineModel::for_workspace(Path::new("/work/zeta"));

    assert_eq!(status_line.text_for_width(80), "/work/zeta");
    assert_eq!(status_line.text_for_width(4), "zeta");
}

#[test]
fn very_narrow_status_line_truncates_on_character_boundaries() {
    let mut status_line = StatusLineModel::for_workspace(Path::new("/work/zeta"));
    status_line.apply_preferred_model(Some(&model("provider", "模型alpha")));

    assert_eq!(status_line.text_for_width(5), "模型…");
    assert_eq!(status_line.text_for_width(1), "…");
    assert_eq!(status_line.text_for_width(0), "");
}

#[test]
fn git_status_adds_branch_and_dirty_state_without_displacing_the_model_first() {
    let mut status_line = StatusLineModel::for_workspace(Path::new("/work/zeta"));
    status_line.apply_preferred_model(Some(&model("anthropic", "claude-sonnet")));
    status_line.apply_git_status(&GitStatusResult {
        stream_instance_id: StreamInstanceId::new("git-stream").unwrap(),
        revision: 3,
        workspace_path: String::new(),
        head: GitHeadDto::Branch {
            name: "main".into(),
            object_id: "0123456789abcdef".into(),
            upstream: None,
        },
        changes: vec![test_change()],
    });

    assert_eq!(
        status_line.text_for_width(100),
        "anthropic/claude-sonnet · git:main (1 changes) · /work/zeta"
    );
    assert_eq!(status_line.text_for_width(23), "claude-sonnet · main*");
}

fn test_change() -> zeta_app_server_protocol::protocol::git::GitRepositoryChangeDto {
    use zeta_app_server_protocol::protocol::git::GitChangeStatusDto;
    use zeta_app_server_protocol::protocol::git::GitSubmoduleStateDto;
    zeta_app_server_protocol::protocol::git::GitRepositoryChangeDto {
        path: "src/lib.rs".into(),
        original_path: None,
        index_status: GitChangeStatusDto::Unmodified,
        worktree_status: GitChangeStatusDto::Modified,
        conflicted: false,
        submodule: GitSubmoduleStateDto {
            is_submodule: false,
            commit_changed: false,
            tracked_changes: false,
            untracked_changes: false,
        },
    }
}

fn model(provider: &str, model: &str) -> ModelRefDto {
    ModelRefDto {
        provider: provider.into(),
        model: model.into(),
    }
}
