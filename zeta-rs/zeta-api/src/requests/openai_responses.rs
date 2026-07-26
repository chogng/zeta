use crate::{
    ApiEndpoint, ApiError, ContentPart, ImageDetail, InputItem, MessageRole, ModelRequest,
    ModelResponse, ModelUsage, OutputItem, ReasoningEffort, StopReason, ToolCall, ToolCallId,
    ToolChoice, ToolDefinition, ToolName,
};
use serde_json::{Map, Value, json};
use zeta_client::{OperationClient, ResolvedApiTarget};

pub(crate) fn complete(
    endpoint: ApiEndpoint,
    target: &ResolvedApiTarget,
    model: &str,
    request: &ModelRequest,
    client: &dyn OperationClient,
) -> Result<ModelResponse, ApiError> {
    let response =
        crate::requests::post_json(client, target, endpoint, build_request(model, request)?)?;
    parse_response(response)
}

fn build_request(model: &str, request: &ModelRequest) -> Result<Value, ApiError> {
    let mut body = Map::from_iter([
        ("model".into(), Value::String(model.into())),
        ("input".into(), Value::Array(convert_input(&request.input)?)),
        ("stream".into(), Value::Bool(false)),
        ("store".into(), Value::Bool(false)),
    ]);
    if let Some(instructions) = &request.instructions {
        body.insert("instructions".into(), Value::String(instructions.clone()));
    }
    if !request.tools.is_empty() {
        body.insert(
            "tools".into(),
            Value::Array(request.tools.iter().map(convert_tool).collect()),
        );
        body.insert(
            "tool_choice".into(),
            convert_responses_tool_choice(&request.tool_choice),
        );
        body.insert(
            "parallel_tool_calls".into(),
            Value::Bool(request.parallel_tool_calls),
        );
    }
    if let Some(reasoning) = &request.reasoning {
        body.insert(
            "reasoning".into(),
            json!({
                "effort": reasoning_effort(reasoning.effort),
                "summary": reasoning.summary.then_some("auto"),
            }),
        );
        if reasoning.summary {
            body.insert("include".into(), json!(["reasoning.encrypted_content"]));
        }
    }
    if let Some(max_output_tokens) = request.max_output_tokens {
        body.insert("max_output_tokens".into(), json!(max_output_tokens));
    }
    if let Some(temperature) = request.temperature {
        body.insert("temperature".into(), json!(temperature));
    }
    Ok(Value::Object(body))
}

fn convert_input(input: &[InputItem]) -> Result<Vec<Value>, ApiError> {
    let mut converted = Vec::new();
    for item in input {
        match item {
            InputItem::Message(message) => {
                if !message.content.is_empty() {
                    converted.push(json!({
                        "role": role(message.role),
                        "content": message
                            .content
                            .iter()
                            .map(|part| convert_content(message.role, part))
                            .collect::<Vec<_>>(),
                    }));
                }
                converted.extend(message.tool_calls.iter().map(|call| {
                    json!({
                        "type": "function_call",
                        "call_id": call.id,
                        "name": call.name,
                        "arguments": call.arguments.to_string(),
                    })
                }));
            }
            InputItem::ToolResult(result) => converted.push(json!({
                "type": "function_call_output",
                "call_id": result.call_id,
                "output": content_text(&result.content),
            })),
        }
    }
    if converted.is_empty() {
        return Err(ApiError::InvalidRequest(
            "input contains no encodable items".into(),
        ));
    }
    Ok(converted)
}

fn convert_content(role: MessageRole, part: &ContentPart) -> Value {
    match part {
        ContentPart::Text(text) => json!({
            "type": if role == MessageRole::Assistant {
                "output_text"
            } else {
                "input_text"
            },
            "text": text,
        }),
        ContentPart::ImageUrl { url, detail } => json!({
            "type": "input_image",
            "image_url": url,
            "detail": image_detail(*detail),
        }),
    }
}

fn convert_tool(tool: &ToolDefinition) -> Value {
    json!({
        "type": "function",
        "name": tool.name,
        "description": tool.description,
        "parameters": tool.parameters,
        "strict": tool.strict,
    })
}

fn convert_responses_tool_choice(choice: &ToolChoice) -> Value {
    match choice {
        ToolChoice::Auto => json!("auto"),
        ToolChoice::None => json!("none"),
        ToolChoice::Required => json!("required"),
        ToolChoice::Function(name) => json!({"type": "function", "name": name}),
    }
}

fn parse_response(response: Value) -> Result<ModelResponse, ApiError> {
    let mut output = Vec::new();
    for item in response
        .get("output")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        match item.get("type").and_then(Value::as_str) {
            Some("message") => {
                for part in item
                    .get("content")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    match part.get("type").and_then(Value::as_str) {
                        Some("output_text") => push_string(&mut output, part, "text", false),
                        Some("refusal") => push_string(&mut output, part, "refusal", true),
                        _ => {}
                    }
                }
            }
            Some("function_call") => output.push(OutputItem::ToolCall(ToolCall {
                id: ToolCallId::new(required_string(item, "call_id")?)
                    .map_err(|error| ApiError::InvalidResponse(error.to_string()))?,
                name: ToolName::new(required_string(item, "name")?)
                    .map_err(|error| ApiError::InvalidResponse(error.to_string()))?,
                arguments: parse_arguments(item.get("arguments"))?,
            })),
            Some("reasoning") => {
                let text = item
                    .get("summary")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|part| part.get("text").and_then(Value::as_str))
                    .collect::<String>();
                if !text.is_empty() {
                    output.push(OutputItem::Reasoning(text));
                }
            }
            _ => {}
        }
    }
    if output.is_empty() {
        return Err(ApiError::InvalidResponse(
            "Responses API returned no supported output items".into(),
        ));
    }
    let status = response
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("completed");
    let stop_reason = if output
        .iter()
        .any(|item| matches!(item, OutputItem::ToolCall(_)))
    {
        StopReason::ToolUse
    } else if output
        .iter()
        .any(|item| matches!(item, OutputItem::Refusal(_)))
    {
        StopReason::Refusal
    } else {
        match status {
            "completed" => StopReason::Completed,
            "incomplete"
                if response
                    .pointer("/incomplete_details/reason")
                    .and_then(Value::as_str)
                    == Some("max_output_tokens") =>
            {
                StopReason::MaxOutputTokens
            }
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
        input_tokens: usage.get("input_tokens")?.as_u64()?,
        output_tokens: usage.get("output_tokens")?.as_u64()?,
        cached_input_tokens: usage
            .pointer("/input_tokens_details/cached_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        reasoning_tokens: usage
            .pointer("/output_tokens_details/reasoning_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
    })
}

fn push_string(output: &mut Vec<OutputItem>, value: &Value, field: &str, refusal: bool) {
    if let Some(text) = value.get(field).and_then(Value::as_str)
        && !text.is_empty()
    {
        output.push(if refusal {
            OutputItem::Refusal(text.into())
        } else {
            OutputItem::Text(text.into())
        });
    }
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
        .ok_or_else(|| ApiError::InvalidResponse(format!("function call is missing {field}")))
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
        ImageDetail::Original => "original",
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
