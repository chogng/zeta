use crate::{
    ApiError, ContentPart, InputItem, JsonHttpTransport, Message, MessageRole, ModelRequest,
    ModelResponse, ModelUsage, OutputItem, ResolvedApiTarget, StopReason, ToolCall, ToolChoice,
    ToolDefinition,
};
use serde_json::{Map, Value, json};

pub(crate) fn complete(
    target: &ResolvedApiTarget,
    model: &str,
    request: &ModelRequest,
    version: &str,
    transport: &dyn JsonHttpTransport,
) -> Result<ModelResponse, ApiError> {
    if request.reasoning.is_some() {
        return Err(ApiError::InvalidRequest(
            "Anthropic reasoning requires a provider-specific thinking configuration".into(),
        ));
    }
    let mut headers = target.headers.clone();
    if !headers
        .iter()
        .any(|header| header.name().eq_ignore_ascii_case("anthropic-version"))
    {
        headers.push(crate::HttpHeader::new("anthropic-version", version));
    }
    let response = transport.post_json(
        &target.endpoint("v1/messages")?,
        &headers,
        build_request(model, request)?,
    )?;
    parse_response(response)
}

fn build_request(model: &str, request: &ModelRequest) -> Result<Value, ApiError> {
    let mut messages = Vec::new();
    for item in &request.input {
        match item {
            InputItem::Message(message) => messages.push(convert_message(message)?),
            InputItem::ToolResult(result) => messages.push(json!({
                "role": "user",
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": result.call_id,
                    "content": content_text(&result.content),
                    "is_error": result.is_error,
                }],
            })),
        }
    }
    let mut body = Map::from_iter([
        ("model".into(), Value::String(model.into())),
        ("messages".into(), Value::Array(messages)),
        (
            "max_tokens".into(),
            json!(request.max_output_tokens.unwrap_or(4096)),
        ),
    ]);
    if let Some(instructions) = &request.instructions {
        body.insert("system".into(), Value::String(instructions.clone()));
    }
    if !request.tools.is_empty() {
        body.insert(
            "tools".into(),
            Value::Array(request.tools.iter().map(convert_tool).collect()),
        );
        body.insert(
            "tool_choice".into(),
            convert_anthropic_tool_choice(&request.tool_choice),
        );
    }
    if let Some(temperature) = request.temperature {
        body.insert("temperature".into(), json!(temperature));
    }
    Ok(Value::Object(body))
}

fn convert_message(message: &Message) -> Result<Value, ApiError> {
    let role = match message.role {
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
        MessageRole::System | MessageRole::Developer => {
            return Err(ApiError::InvalidRequest(
                "Anthropic system instructions belong in ModelRequest.instructions".into(),
            ));
        }
    };
    let mut content = message
        .content
        .iter()
        .map(convert_content)
        .collect::<Result<Vec<_>, _>>()?;
    content.extend(message.tool_calls.iter().map(|call| {
        json!({
            "type": "tool_use",
            "id": call.id,
            "name": call.name,
            "input": call.arguments,
        })
    }));
    Ok(json!({"role": role, "content": content}))
}

fn convert_content(content: &ContentPart) -> Result<Value, ApiError> {
    match content {
        ContentPart::Text(text) => Ok(json!({"type": "text", "text": text})),
        ContentPart::ImageUrl { .. } => Err(ApiError::InvalidRequest(
            "Anthropic image inputs require normalized source data, not an image URL".into(),
        )),
    }
}

fn convert_tool(tool: &ToolDefinition) -> Value {
    json!({
        "name": tool.name,
        "description": tool.description,
        "input_schema": tool.parameters,
    })
}

fn convert_anthropic_tool_choice(choice: &ToolChoice) -> Value {
    match choice {
        ToolChoice::Auto => json!({"type": "auto"}),
        ToolChoice::None => json!({"type": "none"}),
        ToolChoice::Required => json!({"type": "any"}),
        ToolChoice::Function(name) => json!({"type": "tool", "name": name}),
    }
}

fn parse_response(response: Value) -> Result<ModelResponse, ApiError> {
    let mut output = Vec::new();
    for item in response
        .get("content")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        match item.get("type").and_then(Value::as_str) {
            Some("text") => {
                if let Some(text) = item.get("text").and_then(Value::as_str)
                    && !text.is_empty()
                {
                    output.push(OutputItem::Text(text.into()));
                }
            }
            Some("thinking") => {
                if let Some(text) = item.get("thinking").and_then(Value::as_str)
                    && !text.is_empty()
                {
                    output.push(OutputItem::Reasoning(text.into()));
                }
            }
            Some("tool_use") => output.push(OutputItem::ToolCall(ToolCall {
                id: required_string(item, "id")?.into(),
                name: required_string(item, "name")?.into(),
                arguments: item.get("input").cloned().unwrap_or_else(|| json!({})),
            })),
            _ => {}
        }
    }
    if output.is_empty() {
        return Err(ApiError::InvalidResponse(
            "Anthropic returned no supported content blocks".into(),
        ));
    }
    let stop_reason = if output
        .iter()
        .any(|item| matches!(item, OutputItem::ToolCall(_)))
    {
        StopReason::ToolUse
    } else {
        match response
            .get("stop_reason")
            .and_then(Value::as_str)
            .unwrap_or("end_turn")
        {
            "end_turn" | "stop_sequence" => StopReason::Completed,
            "max_tokens" => StopReason::MaxOutputTokens,
            other => StopReason::Other(other.into()),
        }
    };
    Ok(ModelResponse {
        id: response
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_owned),
        output,
        usage: parse_usage(response.get("usage")),
        stop_reason,
    })
}

fn parse_usage(usage: Option<&Value>) -> Option<ModelUsage> {
    let usage = usage?;
    Some(ModelUsage {
        input_tokens: usage.get("input_tokens")?.as_u64()?,
        output_tokens: usage.get("output_tokens")?.as_u64()?,
        cached_input_tokens: usage
            .get("cache_read_input_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        reasoning_tokens: 0,
    })
}

fn required_string<'a>(value: &'a Value, field: &str) -> Result<&'a str, ApiError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::InvalidResponse(format!("tool use is missing {field}")))
}

fn content_text(content: &[ContentPart]) -> String {
    content
        .iter()
        .filter_map(|part| match part {
            ContentPart::Text(text) => Some(text.as_str()),
            ContentPart::ImageUrl { .. } => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}
