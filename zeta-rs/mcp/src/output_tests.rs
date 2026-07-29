use zeta_rmcp_client::{CallToolResult, ContentBlock};
use zeta_tools::ToolOutputStatus;

use super::project_tool_result;
use crate::McpCallError;

#[test]
fn rejects_result_over_byte_limit() {
    let error = project_tool_result(
        CallToolResult::success(vec![ContentBlock::text("oversized")]),
        4,
    )
    .expect_err("oversized result must be rejected");

    assert!(matches!(error, McpCallError::InvalidResult(_)));
}

#[test]
fn preserves_remote_tool_error_as_output_status() {
    let mut result = CallToolResult::success(vec![ContentBlock::text("remote failure")]);
    result.is_error = Some(true);

    let output = project_tool_result(result, 1024).expect("project remote error");

    assert_eq!(output.status(), ToolOutputStatus::Error);
}
