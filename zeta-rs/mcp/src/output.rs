use zeta_rmcp_client::{CallToolResult, ContentBlock};
use zeta_tools::{ImageDetail, ToolContent, ToolOutput};

use crate::McpCallError;

pub(crate) fn project_tool_result(
    result: CallToolResult,
    maximum_bytes: usize,
) -> Result<ToolOutput, McpCallError> {
    let mut content = Vec::new();
    let mut bytes = 0usize;
    for block in result.content {
        let projected = match block {
            ContentBlock::Text(text) => ToolContent::Text(text.text),
            ContentBlock::Image(image) if valid_image_mime(&image.mime_type) => {
                ToolContent::Image {
                    url: format!("data:{};base64,{}", image.mime_type, image.data),
                    detail: ImageDetail::Auto,
                }
            }
            block => ToolContent::Text(
                serde_json::to_string(&block)
                    .map_err(|error| McpCallError::InvalidResult(error.to_string()))?,
            ),
        };
        bytes = bytes
            .checked_add(content_bytes(&projected))
            .ok_or_else(|| McpCallError::InvalidResult("output byte count overflow".into()))?;
        if bytes > maximum_bytes {
            return Err(McpCallError::InvalidResult(
                "tool output byte limit exceeded".into(),
            ));
        }
        content.push(projected);
    }
    if let Some(structured) = result.structured_content {
        let structured = serde_json::to_string(&structured)
            .map_err(|error| McpCallError::InvalidResult(error.to_string()))?;
        bytes = bytes
            .checked_add(structured.len())
            .ok_or_else(|| McpCallError::InvalidResult("output byte count overflow".into()))?;
        if bytes > maximum_bytes {
            return Err(McpCallError::InvalidResult(
                "tool output byte limit exceeded".into(),
            ));
        }
        content.push(ToolContent::Text(structured));
    }
    if result.is_error.unwrap_or(false) {
        Ok(ToolOutput::error(content))
    } else {
        Ok(ToolOutput::success(content))
    }
}

fn valid_image_mime(mime: &str) -> bool {
    mime.starts_with("image/")
        && mime
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'+' | b'.' | b'-'))
}

fn content_bytes(content: &ToolContent) -> usize {
    match content {
        ToolContent::Text(text) => text.len(),
        ToolContent::Image { url, .. } => url.len(),
    }
}

#[cfg(test)]
#[path = "output_tests.rs"]
mod tests;
