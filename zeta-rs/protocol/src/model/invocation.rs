use std::borrow::Cow;

use crate::ReasoningEffort;
use crate::ToolCallId;
use crate::ToolName;
use schemars::JsonSchema;
use schemars::Schema;
use schemars::SchemaGenerator;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;
use serde_json::Value;
use ts_rs::TS;
use ts_rs::TypeVisitor;

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContentPart {
    Text(String),
    ImageUrl { url: String, detail: ImageDetail },
}

#[derive(JsonSchema, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum ContentPartWire {
    Text { text: String },
    ImageUrl { url: String, detail: ImageDetail },
}

#[derive(Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum ContentPartRef<'a> {
    Text { text: &'a str },
    ImageUrl { url: &'a str, detail: ImageDetail },
}

impl Serialize for ContentPart {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Text(text) => ContentPartRef::Text { text }.serialize(serializer),
            Self::ImageUrl { url, detail } => ContentPartRef::ImageUrl {
                url,
                detail: *detail,
            }
            .serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for ContentPart {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        ContentPartWire::deserialize(deserializer).map(Into::into)
    }
}

impl From<ContentPartWire> for ContentPart {
    fn from(part: ContentPartWire) -> Self {
        match part {
            ContentPartWire::Text { text } => Self::Text(text),
            ContentPartWire::ImageUrl { url, detail } => Self::ImageUrl { url, detail },
        }
    }
}

impl JsonSchema for ContentPart {
    fn schema_name() -> Cow<'static, str> {
        "ContentPart".into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        ContentPartWire::json_schema(generator)
    }
}

impl TS for ContentPart {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;

    const IS_ENUM: bool = true;

    fn name(_: &ts_rs::Config) -> String {
        "ContentPart".into()
    }

    fn inline(cfg: &ts_rs::Config) -> String {
        format!(
            "{{ \"type\": \"text\", text: string, }} | {{ \"type\": \"imageUrl\", url: string, detail: {}, }}",
            ImageDetail::name(cfg),
        )
    }

    fn decl(cfg: &ts_rs::Config) -> String {
        format!("type {} = {};", Self::name(cfg), Self::inline(cfg))
    }

    fn decl_concrete(cfg: &ts_rs::Config) -> String {
        Self::decl(cfg)
    }

    fn visit_dependencies(visitor: &mut impl TypeVisitor)
    where
        Self: 'static,
    {
        visitor.visit::<ImageDetail>();
    }
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
