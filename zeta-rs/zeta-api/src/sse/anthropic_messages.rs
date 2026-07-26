use crate::ApiError;
use serde_json::Value;
use std::collections::BTreeMap;
use zeta_client::{SseEvent, SseFrame};
use zeta_protocol::ModelStreamEvent;

/// Decodes already-framed Anthropic Messages events into canonical deltas.
///
/// The decoder validates Anthropic's message/content-block lifecycle and
/// filters protocol heartbeat and tool-argument fragments. It does not own
/// SSE framing, transport liveness, or stream reconnection.
pub struct AnthropicMessagesSseDecoder {
    message_started: bool,
    terminal: bool,
    blocks: BTreeMap<u64, ContentBlockKind>,
}

impl AnthropicMessagesSseDecoder {
    pub fn new() -> Self {
        Self {
            message_started: false,
            terminal: false,
            blocks: BTreeMap::new(),
        }
    }

    pub fn decode(&mut self, frame: &SseFrame) -> Result<Vec<ModelStreamEvent>, ApiError> {
        let SseFrame::Event(event) = frame else {
            return Ok(Vec::new());
        };
        if self.terminal {
            return Err(ApiError::InvalidResponse(
                "Anthropic message stream emitted an event after message_stop".into(),
            ));
        }
        self.decode_event(event)
    }

    /// Verifies that the stream ended after `message_stop` with every content
    /// block closed.
    pub fn finish(self) -> Result<(), ApiError> {
        if self.terminal && self.blocks.is_empty() {
            Ok(())
        } else {
            Err(ApiError::InvalidResponse(
                "Anthropic message stream ended before a valid message_stop".into(),
            ))
        }
    }

    fn decode_event(&mut self, event: &SseEvent) -> Result<Vec<ModelStreamEvent>, ApiError> {
        let payload: Value = serde_json::from_str(&event.data).map_err(|_| {
            ApiError::InvalidResponse("Anthropic message stream event contains invalid JSON".into())
        })?;
        let event_type = payload
            .get("type")
            .and_then(Value::as_str)
            .or(event.event.as_deref())
            .ok_or_else(|| {
                ApiError::InvalidResponse(
                    "Anthropic message stream event is missing its type".into(),
                )
            })?;

        match event_type {
            "message_start" => self.start_message(),
            "content_block_start" => self.start_block(&payload),
            "content_block_delta" => self.decode_block_delta(&payload),
            "content_block_stop" => self.stop_block(&payload),
            "message_stop" => self.stop_message(),
            "ping" | "message_delta" => Ok(Vec::new()),
            "error" => Err(ApiError::InvalidResponse(
                "Anthropic message stream reported an error event".into(),
            )),
            _ => Ok(Vec::new()),
        }
    }

    fn start_message(&mut self) -> Result<Vec<ModelStreamEvent>, ApiError> {
        if self.message_started {
            return Err(ApiError::InvalidResponse(
                "Anthropic message stream emitted message_start more than once".into(),
            ));
        }
        self.message_started = true;
        Ok(Vec::new())
    }

    fn start_block(&mut self, payload: &Value) -> Result<Vec<ModelStreamEvent>, ApiError> {
        self.require_message_started()?;
        let index = required_index(payload)?;
        let block_type = payload
            .get("content_block")
            .and_then(|block| block.get("type"))
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ApiError::InvalidResponse(
                    "Anthropic content_block_start is missing content_block.type".into(),
                )
            })?;
        if self
            .blocks
            .insert(index, ContentBlockKind::from(block_type))
            .is_some()
        {
            return Err(ApiError::InvalidResponse(
                "Anthropic message stream started a content block twice".into(),
            ));
        }
        Ok(Vec::new())
    }

    fn decode_block_delta(&self, payload: &Value) -> Result<Vec<ModelStreamEvent>, ApiError> {
        self.require_message_started()?;
        let index = required_index(payload)?;
        let block_kind = self.blocks.get(&index).ok_or_else(|| {
            ApiError::InvalidResponse(
                "Anthropic message stream emitted a delta before content_block_start".into(),
            )
        })?;
        let delta = payload.get("delta").ok_or_else(|| {
            ApiError::InvalidResponse("Anthropic content_block_delta is missing delta".into())
        })?;
        let delta_type = delta.get("type").and_then(Value::as_str).ok_or_else(|| {
            ApiError::InvalidResponse("Anthropic content_block_delta is missing delta.type".into())
        })?;

        match (block_kind, delta_type) {
            (ContentBlockKind::Text, "text_delta") => Ok(vec![ModelStreamEvent::TextDelta(
                required_string(delta, "text", "Anthropic text_delta")?.into(),
            )]),
            (ContentBlockKind::Thinking, "thinking_delta") => {
                Ok(vec![ModelStreamEvent::ReasoningDelta(
                    required_string(delta, "thinking", "Anthropic thinking_delta")?.into(),
                )])
            }
            (ContentBlockKind::Thinking, "signature_delta")
            | (ContentBlockKind::ToolUse, "input_json_delta")
            | (ContentBlockKind::Other, _) => Ok(Vec::new()),
            _ => Err(ApiError::InvalidResponse(
                "Anthropic content block received an incompatible delta type".into(),
            )),
        }
    }

    fn stop_block(&mut self, payload: &Value) -> Result<Vec<ModelStreamEvent>, ApiError> {
        self.require_message_started()?;
        let index = required_index(payload)?;
        if self.blocks.remove(&index).is_none() {
            return Err(ApiError::InvalidResponse(
                "Anthropic message stream stopped an unknown content block".into(),
            ));
        }
        Ok(Vec::new())
    }

    fn stop_message(&mut self) -> Result<Vec<ModelStreamEvent>, ApiError> {
        self.require_message_started()?;
        if !self.blocks.is_empty() {
            return Err(ApiError::InvalidResponse(
                "Anthropic message stream reached message_stop with an open content block".into(),
            ));
        }
        self.terminal = true;
        Ok(Vec::new())
    }

    fn require_message_started(&self) -> Result<(), ApiError> {
        if self.message_started {
            Ok(())
        } else {
            Err(ApiError::InvalidResponse(
                "Anthropic message stream emitted a content event before message_start".into(),
            ))
        }
    }
}

#[derive(Clone, Copy)]
enum ContentBlockKind {
    Text,
    Thinking,
    ToolUse,
    Other,
}

impl ContentBlockKind {
    fn from(block_type: &str) -> Self {
        match block_type {
            "text" => Self::Text,
            "thinking" => Self::Thinking,
            "tool_use" => Self::ToolUse,
            _ => Self::Other,
        }
    }
}

fn required_index(payload: &Value) -> Result<u64, ApiError> {
    payload.get("index").and_then(Value::as_u64).ok_or_else(|| {
        ApiError::InvalidResponse("Anthropic stream event is missing content block index".into())
    })
}

fn required_string<'a>(
    value: &'a Value,
    field: &str,
    event_type: &str,
) -> Result<&'a str, ApiError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::InvalidResponse(format!("{event_type} is missing {field}")))
}
