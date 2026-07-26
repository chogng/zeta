use crate::{CoreError, ThreadSnapshot};
use std::collections::BTreeMap;
use zeta_protocol::{
    ContentPart, InputItem, Message, MessageRole, ModelRequest, ThreadItem, ToolCall, ToolChoice,
    ToolDefinition, ToolResult,
};

/// Derives one provider-independent model request from durable Thread history.
pub(crate) struct ContextAssembler;

impl ContextAssembler {
    pub(crate) fn assemble(
        snapshot: &ThreadSnapshot,
        tools: Vec<ToolDefinition>,
    ) -> Result<ModelRequest, CoreError> {
        let mut input = Vec::new();
        let mut tool_names = BTreeMap::new();

        for item in &snapshot.items {
            match item {
                ThreadItem::UserMessage { text, .. } => {
                    input.push(InputItem::Message(Message::text(
                        MessageRole::User,
                        text.clone(),
                    )));
                }
                ThreadItem::AgentMessage { text, .. } => {
                    input.push(InputItem::Message(Message::text(
                        MessageRole::Assistant,
                        text.clone(),
                    )));
                }
                ThreadItem::ToolCall {
                    tool_call_id,
                    name,
                    arguments_json,
                    ..
                } => {
                    let arguments = serde_json::from_str(arguments_json).map_err(|error| {
                        CoreError::Context(format!(
                            "Tool Call {tool_call_id} contains invalid JSON arguments: {error}"
                        ))
                    })?;
                    let call = ToolCall {
                        id: tool_call_id.clone(),
                        name: name.clone(),
                        arguments,
                    };
                    tool_names.insert(tool_call_id.clone(), name.clone());
                    append_tool_call(&mut input, call);
                }
                ThreadItem::ToolResult {
                    tool_call_id,
                    text,
                    is_error,
                    ..
                } => {
                    let name = tool_names.get(tool_call_id).cloned().ok_or_else(|| {
                        CoreError::Context(format!(
                            "Tool Result references an unavailable Tool Call: {tool_call_id}"
                        ))
                    })?;
                    input.push(InputItem::ToolResult(ToolResult {
                        call_id: tool_call_id.clone(),
                        name,
                        content: vec![ContentPart::Text(text.clone())],
                        is_error: *is_error,
                    }));
                }
                // Reasoning and plan items are durable product output, not provider-neutral
                // conversation messages. They require an explicit provider contract before they
                // can safely be fed back into another invocation.
                ThreadItem::Reasoning { .. } | ThreadItem::Plan { .. } => {}
            }
        }

        if input.is_empty() {
            return Err(CoreError::Context(
                "cannot invoke a model without durable Thread input".into(),
            ));
        }

        let tool_choice = if tools.is_empty() {
            ToolChoice::None
        } else {
            ToolChoice::Auto
        };
        Ok(ModelRequest {
            instructions: None,
            input,
            tools,
            tool_choice,
            parallel_tool_calls: false,
            reasoning: None,
            max_output_tokens: None,
            temperature: None,
        })
    }
}

fn append_tool_call(input: &mut Vec<InputItem>, call: ToolCall) {
    if let Some(InputItem::Message(message)) = input.last_mut()
        && message.role == MessageRole::Assistant
    {
        message.tool_calls.push(call);
        return;
    }
    input.push(InputItem::Message(Message {
        role: MessageRole::Assistant,
        content: Vec::new(),
        tool_calls: vec![call],
    }));
}

#[cfg(test)]
#[path = "assembler_tests.rs"]
mod tests;
