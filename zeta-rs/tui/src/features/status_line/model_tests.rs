use super::*;
use std::collections::BTreeMap;
use std::path::Path;
use zeta_app_server_protocol::protocol::config::{
    ApprovalReviewModelSelectionDto, ConfigReadResult, ModelRefDto,
};

#[test]
fn wide_status_line_prefers_full_model_and_workspace_values() {
    let mut status_line = StatusLineModel::for_workspace(Path::new("/work/zeta"));
    status_line.apply_config(&config_with_model("anthropic", "claude-sonnet"));

    assert_eq!(
        status_line.text_for_width(80),
        "anthropic/claude-sonnet · /work/zeta"
    );
}

#[test]
fn narrow_status_line_uses_compact_values_before_hiding_workspace() {
    let mut status_line = StatusLineModel::for_workspace(Path::new("/work/zeta"));
    status_line.apply_config(&config_with_model("anthropic", "claude-sonnet"));

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
    status_line.apply_config(&config_with_model("provider", "模型alpha"));

    assert_eq!(status_line.text_for_width(5), "模型…");
    assert_eq!(status_line.text_for_width(1), "…");
    assert_eq!(status_line.text_for_width(0), "");
}

fn config_with_model(provider: &str, model: &str) -> ConfigReadResult {
    ConfigReadResult {
        revision: 0,
        generation: 0,
        preferred_model: Some(ModelRefDto {
            provider: provider.into(),
            model: model.into(),
        }),
        approval_review_model: ApprovalReviewModelSelectionDto::Automatic,
        providers: BTreeMap::new(),
        mcp_servers: BTreeMap::new(),
        skill_sources: BTreeMap::new(),
        plugin_requests: BTreeMap::new(),
        hooks: BTreeMap::new(),
    }
}
