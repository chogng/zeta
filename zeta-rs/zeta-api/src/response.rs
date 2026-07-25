use crate::ToolCall;

#[derive(Clone, Debug, PartialEq)]
pub struct ModelResponse {
    pub id: Option<String>,
    pub output: Vec<OutputItem>,
    pub usage: Option<ModelUsage>,
    pub stop_reason: StopReason,
}

impl ModelResponse {
    pub fn text(&self) -> String {
        self.output
            .iter()
            .filter_map(|item| match item {
                OutputItem::Text(text) => Some(text.as_str()),
                _ => None,
            })
            .collect()
    }

    pub fn tool_calls(&self) -> impl Iterator<Item = &ToolCall> {
        self.output.iter().filter_map(|item| match item {
            OutputItem::ToolCall(call) => Some(call),
            _ => None,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum OutputItem {
    Text(String),
    Refusal(String),
    Reasoning(String),
    ToolCall(ToolCall),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_input_tokens: u64,
    pub reasoning_tokens: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StopReason {
    Completed,
    ToolUse,
    MaxOutputTokens,
    Refusal,
    Other(String),
}
