//! Provider event-schema decoders.
//!
//! `zeta-client` owns byte-stream framing and exposes provider-neutral
//! `SseFrame` values. This module owns API event lifecycle interpretation; it
//! deliberately contains no HTTP, retry, buffering, or heartbeat timeout
//! behavior.

mod anthropic_messages;
mod openai_responses;

pub use anthropic_messages::AnthropicMessagesSseDecoder;
pub use openai_responses::OpenAiResponsesSseDecoder;

#[cfg(test)]
#[path = "anthropic_messages_tests.rs"]
mod anthropic_messages_tests;

#[cfg(test)]
#[path = "openai_responses_tests.rs"]
mod openai_responses_tests;
