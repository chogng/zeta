use crate::{
    ApiEndpoint, ApiError, ContentPart, ImageDetail, InputItem, Message, MessageRole, ModelRequest,
    ModelResponse, ModelUsage, OutputItem, ReasoningEffort, StopReason, ToolCall, ToolCallId,
    ToolChoice, ToolDefinition, ToolName,
};
use serde_json::{Map, Value, json};
use zeta_async_utils::CancellationToken;
use zeta_client::{OperationClient, ResolvedApiTarget};

pub(crate) fn complete(
    endpoint: ApiEndpoint,
    target: &ResolvedApiTarget,
    model: &str,
    request: &ModelRequest,
    client: &dyn OperationClient,
    cancellation: &CancellationToken,
) -> Result<ModelResponse, ApiError> {
    let response = crate::requests::post_json(
        client,
        target,
        endpoint,
        build_request(model, request)?,
        cancellation,
    )?;
    parse_response(response)
}

pub(crate) fn build_request(model: &str, request: &ModelRequest) -> Result<Value, ApiError> {
    let mut messages = Vec::new();
    if let Some(instructions) = &request.instructions {
        messages.push(json!({"role": "system", "content": instructions}));
    }
    for item in &request.input {
        match item {
            InputItem::Message(message) => messages.push(convert_message(message)),
            InputItem::ToolResult(result) => messages.push(json!({
                "role": "tool",
                "tool_call_id": result.call_id,
                "content": content_text(&result.content),
            })),
        }
    }
    let mut body = Map::from_iter([
        ("model".into(), Value::String(model.into())),
        ("messages".into(), Value::Array(messages)),
        ("stream".into(), Value::Bool(false)),
    ]);
    if !request.tools.is_empty() {
        body.insert(
            "tools".into(),
            Value::Array(request.tools.iter().map(convert_tool).collect()),
        );
        body.insert(
            "tool_choice".into(),
            convert_completions_tool_choice(&request.tool_choice),
        );
        body.insert(
            "parallel_tool_calls".into(),
            Value::Bool(request.parallel_tool_calls),
        );
    }
    if let Some(reasoning) = &request.reasoning {
        body.insert(
            "reasoning_effort".into(),
            json!(reasoning_effort(reasoning.effort)),
        );
    }
    if let Some(max_output_tokens) = request.max_output_tokens {
        body.insert("max_tokens".into(), json!(max_output_tokens));
    }
    if let Some(temperature) = request.temperature {
        body.insert("temperature".into(), json!(temperature));
    }
    Ok(Value::Object(body))
}

fn convert_message(message: &Message) -> Value {
    let mut converted = Map::from_iter([
        ("role".into(), json!(role(message.role))),
        ("content".into(), convert_content(&message.content)),
    ]);
    if !message.tool_calls.is_empty() {
        converted.insert(
            "tool_calls".into(),
            Value::Array(
                message
                    .tool_calls
                    .iter()
                    .map(|call| {
                        json!({
                            "id": call.id,
                            "type": "function",
                            "function": {
                                "name": call.name,
                                "arguments": call.arguments.to_string(),
                            },
                        })
                    })
                    .collect(),
            ),
        );
    }
    Value::Object(converted)
}

fn convert_content(content: &[ContentPart]) -> Value {
    if content.len() == 1
        && let ContentPart::Text(text) = &content[0]
    {
        return Value::String(text.clone());
    }
    Value::Array(
        content
            .iter()
            .map(|part| match part {
                ContentPart::Text(text) => json!({"type": "text", "text": text}),
                ContentPart::ImageAttachment { .. } => {
                    unreachable!("durable image attachments must be materialized before API encoding")
                }
                ContentPart::ImageUrl { url, detail } => json!({
                    "type": "image_url",
                    "image_url": {
                        "url": url,
                        "detail": image_detail(*detail),
                    },
                }),
            })
            .collect(),
    )
}

fn convert_tool(tool: &ToolDefinition) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": tool.name,
            "description": tool.description,
            "parameters": tool.parameters,
            "strict": tool.strict,
        },
    })
}

fn convert_completions_tool_choice(choice: &ToolChoice) -> Value {
    match choice {
        ToolChoice::Auto => json!("auto"),
        ToolChoice::None => json!("none"),
        ToolChoice::Required => json!("required"),
        ToolChoice::Function(name) => json!({
            "type": "function",
            "function": {"name": name},
        }),
    }
}

fn parse_response(response: Value) -> Result<ModelResponse, ApiError> {
    let choice = response
        .pointer("/choices/0")
        .ok_or_else(|| ApiError::InvalidResponse("missing first completion choice".into()))?;
    let message = choice
        .get("message")
        .ok_or_else(|| ApiError::InvalidResponse("completion choice is missing message".into()))?;
    let mut output = Vec::new();
    if let Some(content) = message.get("content") {
        match content {
            Value::String(text) if !text.is_empty() => output.push(OutputItem::Text(text.clone())),
            Value::Array(parts) => {
                for part in parts {
                    if let Some(text) = part.get("text").and_then(Value::as_str)
                        && !text.is_empty()
                    {
                        output.push(OutputItem::Text(text.into()));
                    }
                }
            }
            _ => {}
        }
    }
    if let Some(reasoning) = message
        .get("reasoning_content")
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
    {
        output.push(OutputItem::Reasoning(reasoning.into()));
    }
    for call in message
        .get("tool_calls")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        output.push(OutputItem::ToolCall(ToolCall {
            id: ToolCallId::new(required_string(call, "id")?)
                .map_err(|error| ApiError::InvalidResponse(error.to_string()))?,
            name: ToolName::new(
                call.pointer("/function/name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        ApiError::InvalidResponse("tool call is missing function name".into())
                    })?,
            )
            .map_err(|error| ApiError::InvalidResponse(error.to_string()))?,
            arguments: parse_arguments(call.pointer("/function/arguments"))?,
        }));
    }
    if output.is_empty() {
        return Err(ApiError::InvalidResponse(
            "completion returned no supported output items".into(),
        ));
    }
    let finish_reason = choice
        .get("finish_reason")
        .and_then(Value::as_str)
        .unwrap_or("stop");
    let stop_reason = if output
        .iter()
        .any(|item| matches!(item, OutputItem::ToolCall(_)))
    {
        StopReason::ToolUse
    } else {
        match finish_reason {
            "stop" => StopReason::Completed,
            "length" => StopReason::MaxOutputTokens,
            other => StopReason::Other(other.into()),
        }
    };
    Ok(ModelResponse {
        output,
        usage: parse_usage(response.get("usage")),
        stop_reason,
    })
}

fn parse_usage(usage: Option<&Value>) -> Option<ModelUsage> {
    let usage = usage?;
    Some(ModelUsage {
        input_tokens: usage.get("prompt_tokens")?.as_u64()?,
        output_tokens: usage.get("completion_tokens")?.as_u64()?,
        cached_input_tokens: usage
            .pointer("/prompt_tokens_details/cached_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        reasoning_tokens: usage
            .pointer("/completion_tokens_details/reasoning_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
    })
}

fn parse_arguments(arguments: Option<&Value>) -> Result<Value, ApiError> {
    match arguments {
        Some(Value::String(arguments)) => serde_json::from_str(arguments)
            .map_err(|_| ApiError::InvalidResponse("tool arguments are invalid JSON".into())),
        Some(arguments) => Ok(arguments.clone()),
        None => Ok(json!({})),
    }
}

fn required_string<'a>(value: &'a Value, field: &str) -> Result<&'a str, ApiError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::InvalidResponse(format!("tool call is missing {field}")))
}

fn content_text(content: &[ContentPart]) -> String {
    content
        .iter()
        .filter_map(|part| match part {
            ContentPart::Text(text) => Some(text.as_str()),
            ContentPart::ImageAttachment { .. } => None,
            ContentPart::ImageUrl { .. } => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn role(role: MessageRole) -> &'static str {
    match role {
        MessageRole::System => "system",
        MessageRole::Developer => "developer",
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
    }
}

fn image_detail(detail: ImageDetail) -> &'static str {
    match detail {
        ImageDetail::Auto => "auto",
        ImageDetail::Low => "low",
        ImageDetail::High => "high",
        ImageDetail::Original => "auto",
    }
}

fn reasoning_effort(effort: ReasoningEffort) -> &'static str {
    match effort {
        ReasoningEffort::None => "none",
        ReasoningEffort::Minimal => "minimal",
        ReasoningEffort::Low => "low",
        ReasoningEffort::Medium => "medium",
        ReasoningEffort::High => "high",
        ReasoningEffort::ExtraHigh => "xhigh",
        ReasoningEffort::Max => "max",
    }
}
