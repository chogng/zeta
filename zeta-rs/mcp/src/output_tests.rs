use zeta_rmcp_client::{CallToolResult, ContentBlock};
use zeta_tools::{ToolContent, ToolOutputStatus};

use super::project_tool_result;
use crate::McpCallError;

#[test]
fn truncates_text_result_at_byte_limit() {
    let output = project_tool_result(
        CallToolResult::success(vec![ContentBlock::text("oversized output ".repeat(64))]),
        128,
    )
    .expect("oversized text should be truncated");

    assert!(matches!(
        output.content(),
        [ToolContent::Text(text)]
            if text.len() <= 128 && text.contains("Warning: truncated output")
    ));
}

#[test]
fn rejects_image_result_that_cannot_fit_the_byte_limit() {
    let error = project_tool_result(
        CallToolResult::success(vec![ContentBlock::image("AA==", "image/png")]),
        8,
    )
    .expect_err("an image must not be cut into an invalid data URL");

    assert!(matches!(
        error,
        McpCallError::InvalidResult(message) if message.contains("image byte limit")
    ));
}

#[test]
fn preserves_remote_tool_error_as_output_status() {
    let mut result = CallToolResult::success(vec![ContentBlock::text("remote failure")]);
    result.is_error = Some(true);

    let output = project_tool_result(result, 1024).expect("project remote error");

    assert_eq!(output.status(), ToolOutputStatus::Error);
}
