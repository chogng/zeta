use super::*;
use crate::ApiError;
use zeta_client::{SseEvent, SseFrame};
use zeta_protocol::ModelStreamEvent;

fn event(event: &str, data: &str) -> SseFrame {
    SseFrame::Event(SseEvent {
        event: Some(event.into()),
        data: data.into(),
        id: None,
        retry: None,
    })
}

#[test]
fn responses_decoder_emits_text_and_reasoning_deltas() {
    let mut decoder = OpenAiResponsesSseDecoder::new();

    assert_eq!(
        decoder
            .decode(&event(
                "response.output_text.delta",
                r#"{"type":"response.output_text.delta","delta":"Hello"}"#,
            ))
            .unwrap(),
        vec![ModelStreamEvent::TextDelta("Hello".into())]
    );
    assert_eq!(
        decoder
            .decode(&event(
                "response.reasoning_summary_text.delta",
                r#"{"type":"response.reasoning_summary_text.delta","delta":"Thinking"}"#,
            ))
            .unwrap(),
        vec![ModelStreamEvent::ReasoningDelta("Thinking".into())]
    );
    decoder
        .decode(&event(
            "response.completed",
            r#"{"type":"response.completed"}"#,
        ))
        .unwrap();
    decoder.finish().unwrap();
}

#[test]
fn responses_decoder_ignores_comments_and_unknown_optional_events() {
    let mut decoder = OpenAiResponsesSseDecoder::new();
    assert!(decoder.decode(&SseFrame::Comment).unwrap().is_empty());
    assert!(
        decoder
            .decode(&event("response.created", r#"{"type":"response.created"}"#,))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn responses_decoder_rejects_eof_before_a_terminal_event() {
    let decoder = OpenAiResponsesSseDecoder::new();
    assert!(matches!(
        decoder.finish(),
        Err(ApiError::InvalidResponse(_))
    ));
}

#[test]
fn responses_decoder_rejects_malformed_delta_events() {
    let mut decoder = OpenAiResponsesSseDecoder::new();
    assert!(matches!(
        decoder.decode(&event(
            "response.output_text.delta",
            r#"{"type":"response.output_text.delta"}"#,
        )),
        Err(ApiError::InvalidResponse(_))
    ));
}

#[test]
fn responses_decoder_classifies_terminal_provider_failures() {
    let mut decoder = OpenAiResponsesSseDecoder::new();
    assert!(matches!(
        decoder.decode(&event(
            "response.failed",
            r#"{"type":"response.failed","response":{"error":{"code":"context_length_exceeded"}}}"#,
        )),
        Err(ApiError::ContextOverflow(_))
    ));
}
