use crate::{ReasoningEffort, ToolCallId, ToolName};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRequest {
    pub instructions: Option<String>,
    pub input: Vec<InputItem>,
    pub tools: Vec<ToolDefinition>,
    pub tool_choice: ToolChoice,
    pub parallel_tool_calls: bool,
    pub reasoning: Option<ReasoningConfig>,
    pub max_output_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

impl ModelRequest {
    /// Applies the last provider-capability gate to every image before wire encoding.
    ///
    /// The returned decisions are suitable for bounded diagnostics; image URLs and bytes are not
    /// copied into them.
    pub fn sanitize_image_details(
        &mut self,
        supports_original: bool,
    ) -> Vec<crate::ImageDetailDecision> {
        let mut decisions = Vec::new();
        for item in &mut self.input {
            match item {
                InputItem::Message(message) => {
                    sanitize_content(&mut message.content, supports_original, &mut decisions)
                }
                InputItem::ToolResult(result) => {
                    sanitize_content(&mut result.content, supports_original, &mut decisions)
                }
            }
        }
        decisions
    }
}

fn sanitize_content(
    content: &mut [ContentPart],
    supports_original: bool,
    decisions: &mut Vec<crate::ImageDetailDecision>,
) {
    for part in content {
        let ContentPart::ImageUrl { detail, .. } = part else {
            continue;
        };
        let requested = *detail;
        let (effective, reason) = if requested == ImageDetail::Original && !supports_original {
            (
                ImageDetail::Auto,
                crate::ImageDetailDecisionReason::OriginalUnsupportedDowngraded,
            )
        } else {
            (requested, crate::ImageDetailDecisionReason::Supported)
        };
        *detail = effective;
        decisions.push(crate::ImageDetailDecision {
            requested,
            effective,
            reason,
        });
    }
}

impl ModelRequest {
    pub fn text(prompt: impl Into<String>) -> Self {
        Self {
            instructions: None,
            input: vec![InputItem::Message(Message::text(MessageRole::User, prompt))],
            tools: Vec::new(),
            tool_choice: ToolChoice::Auto,
            parallel_tool_calls: true,
            reasoning: None,
            max_output_tokens: None,
            temperature: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum InputItem {
    Message(Message),
    ToolResult(ToolResult),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub role: MessageRole,
    pub content: Vec<ContentPart>,
    pub tool_calls: Vec<ToolCall>,
}

impl Message {
    pub fn text(role: MessageRole, text: impl Into<String>) -> Self {
        Self {
            role,
            content: vec![ContentPart::Text(text.into())],
            tool_calls: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MessageRole {
    System,
    Developer,
    User,
    Assistant,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ContentPart {
    Text(String),
    ImageUrl { url: String, detail: ImageDetail },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ImageDetail {
    Auto,
    Low,
    High,
    Original,
}

/// Stable explanation for a final model-request image detail decision.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ImageDetailDecisionReason {
    Supported,
    OriginalUnsupportedDowngraded,
}

/// Observable result of the final model-request image detail gate.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageDetailDecision {
    pub requested: ImageDetail,
    pub effective: ImageDetail,
    pub reason: ImageDetailDecisionReason,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDefinition {
    pub name: ToolName,
    pub description: String,
    pub parameters: Value,
    pub strict: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    pub id: ToolCallId,
    pub name: ToolName,
    pub arguments: Value,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResult {
    pub call_id: ToolCallId,
    pub name: ToolName,
    pub content: Vec<ContentPart>,
    pub is_error: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", content = "name", rename_all = "camelCase")]
pub enum ToolChoice {
    Auto,
    None,
    Required,
    Function(ToolName),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningConfig {
    pub effort: ReasoningEffort,
    pub summary: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelResponse {
    pub output: Vec<ResponseItem>,
    pub usage: Option<ModelUsage>,
    pub stop_reason: StopReason,
}

impl ModelResponse {
    pub fn text(&self) -> String {
        self.output
            .iter()
            .filter_map(|item| match item {
                ResponseItem::Text(text) => Some(text.as_str()),
                _ => None,
            })
            .collect()
    }

    pub fn tool_calls(&self) -> impl Iterator<Item = &ToolCall> {
        self.output.iter().filter_map(|item| match item {
            ResponseItem::ToolCall(call) => Some(call),
            _ => None,
        })
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum ResponseItem {
    Text(String),
    Refusal(String),
    Reasoning(String),
    ToolCall(ToolCall),
}

/// A provider-neutral incremental update produced while a model invocation is in progress.
///
/// Each value contains only newly produced content. The final [`ModelResponse`] remains the
/// authoritative invocation outcome and carries Tool Calls, usage, and stop reason.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", content = "text", rename_all = "camelCase")]
pub enum ModelStreamEvent {
    TextDelta(String),
    ReasoningDelta(String),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_input_tokens: u64,
    pub reasoning_tokens: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", content = "detail", rename_all = "camelCase")]
pub enum StopReason {
    Completed,
    ToolUse,
    MaxOutputTokens,
    Refusal,
    Other(String),
}
