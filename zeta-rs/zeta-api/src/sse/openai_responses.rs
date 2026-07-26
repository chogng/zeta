use crate::ApiError;
use serde_json::Value;
use zeta_client::{SseEvent, SseFrame};
use zeta_protocol::ModelStreamEvent;

/// Decodes already-framed OpenAI Responses events into canonical deltas.
///
/// This decoder owns event-schema validation and terminal lifecycle checks. It
/// intentionally does not own connection state, SSE framing, retry, or stream
/// resumption; those remain client/runtime concerns.
pub struct OpenAiResponsesSseDecoder {
    terminal: bool,
}

impl OpenAiResponsesSseDecoder {
    pub fn new() -> Self {
        Self { terminal: false }
    }

    pub fn decode(&mut self, frame: &SseFrame) -> Result<Vec<ModelStreamEvent>, ApiError> {
        let SseFrame::Event(event) = frame else {
            return Ok(Vec::new());
        };
        if self.terminal {
            return Err(ApiError::InvalidResponse(
                "OpenAI response stream emitted an event after its terminal event".into(),
            ));
        }
        self.decode_event(event)
    }

    /// Verifies that the stream ended after a protocol terminal event.
    pub fn finish(self) -> Result<(), ApiError> {
        if self.terminal {
            Ok(())
        } else {
            Err(ApiError::InvalidResponse(
                "OpenAI response stream ended before a terminal event".into(),
            ))
        }
    }

    fn decode_event(&mut self, event: &SseEvent) -> Result<Vec<ModelStreamEvent>, ApiError> {
        let payload: Value = serde_json::from_str(&event.data).map_err(|_| {
            ApiError::InvalidResponse("OpenAI response stream event contains invalid JSON".into())
        })?;
        let event_type = payload
            .get("type")
            .and_then(Value::as_str)
            .or(event.event.as_deref())
            .ok_or_else(|| {
                ApiError::InvalidResponse("OpenAI response stream event is missing its type".into())
            })?;

        match event_type {
            "response.output_text.delta" => Ok(vec![ModelStreamEvent::TextDelta(
                required_delta(&payload, event_type)?.into(),
            )]),
            "response.reasoning_summary_text.delta" => Ok(vec![ModelStreamEvent::ReasoningDelta(
                required_delta(&payload, event_type)?.into(),
            )]),
            "response.completed" => {
                self.terminal = true;
                Ok(Vec::new())
            }
            "response.failed" | "response.incomplete" => Err(ApiError::InvalidResponse(
                "OpenAI response stream reported a terminal failure".into(),
            )),
            _ => Ok(Vec::new()),
        }
    }
}

fn required_delta<'a>(payload: &'a Value, event_type: &str) -> Result<&'a str, ApiError> {
    payload.get("delta").and_then(Value::as_str).ok_or_else(|| {
        ApiError::InvalidResponse(format!(
            "OpenAI stream event '{event_type}' is missing delta"
        ))
    })
}
