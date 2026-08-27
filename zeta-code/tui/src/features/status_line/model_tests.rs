use super::*;
use zeta_app_server_protocol::protocol::config::ModelRefDto;
use zeta_app_server_protocol::protocol::git::GitHeadDto;
use zeta_app_server_protocol::protocol::git::GitStatusResult;
use zeta_protocol::ApprovalMode;
use zeta_protocol::StreamInstanceId;

#[test]
fn status_line_orders_permissions_model_branch_and_changes() {
    let mut status_line = StatusLineModel::new();
    status_line.apply_preferred_model(Some(&model("anthropic", "claude-sonnet")));
    status_line.apply_git_status(&git_status(1));

    assert_eq!(
        status_line.text_for_width(100, ApprovalMode::AskPermissions),
        "◉ ask permissions on · anthropic/claude-sonnet · main · 1 change"
    );
}

#[test]
fn configured_items_can_be_hidden_independently() {
    let mut status_line = StatusLineModel::new();
    status_line.apply_preferred_model(Some(&model("anthropic", "claude-sonnet")));
    status_line.apply_git_status(&git_status(1));
    let mut settings = StatusLineSettings::default();
    settings.set(StatusLineItem::Permissions, false);
    settings.set(StatusLineItem::GitBranch, false);
    status_line.apply_settings(settings);

    assert_eq!(
        status_line.text_for_width(80, ApprovalMode::AutoReview),
        "anthropic/claude-sonnet · 1 change"
    );
}

#[test]
fn narrow_status_line_keeps_configured_order_and_uses_compact_values() {
    let mut status_line = StatusLineModel::new();
    status_line.apply_preferred_model(Some(&model("anthropic", "claude-sonnet")));
    status_line.apply_git_status(&git_status(1));
    let mut settings = StatusLineSettings::default();
    settings.set(StatusLineItem::Permissions, false);
    status_line.apply_settings(settings);

    assert_eq!(
        status_line.text_for_width(24, ApprovalMode::AskPermissions),
        "claude-sonnet · main · *"
    );
}

#[test]
fn status_line_with_every_item_disabled_is_empty() {
    let mut status_line = StatusLineModel::new();
    let mut settings = StatusLineSettings::default();
    for item in StatusLineItem::ALL {
        settings.set(item, false);
    }
    status_line.apply_settings(settings);

    assert_eq!(
        status_line.text_for_width(80, ApprovalMode::AskPermissions),
        ""
    );
}

#[test]
fn very_narrow_status_line_truncates_on_character_boundaries() {
    let mut status_line = StatusLineModel::new();
    status_line.apply_preferred_model(Some(&model("provider", "模型alpha")));
    let mut settings = StatusLineSettings::default();
    for item in StatusLineItem::ALL {
        settings.set(item, item == StatusLineItem::Model);
    }
    status_line.apply_settings(settings);

    assert_eq!(
        status_line.text_for_width(5, ApprovalMode::AskPermissions),
        "模型…"
    );
    assert_eq!(
        status_line.text_for_width(1, ApprovalMode::AskPermissions),
        "…"
    );
    assert_eq!(
        status_line.text_for_width(0, ApprovalMode::AskPermissions),
        ""
    );
}

fn git_status(change_count: usize) -> GitStatusResult {
    GitStatusResult {
        repository_id: "repository-1".into(),
        stream_instance_id: StreamInstanceId::new("git-stream").unwrap(),
        revision: 3,
        workspace_path: String::new(),
        head: GitHeadDto::Branch {
            name: "main".into(),
            object_id: "0123456789abcdef".into(),
            upstream: None,
        },
        changes: (0..change_count).map(|_| test_change()).collect(),
    }
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
