//! Normalized model requests and provider wire-protocol conversions.

mod endpoint;
mod error;
mod input_token_count_endpoint;
mod requests;
mod semantic;
mod sse;
mod token_count;

pub use endpoint::ApiEndpoint;
pub use endpoint::ApiProtocol;
pub use endpoint::ApiStreamSink;
pub use error::ApiError;
pub use input_token_count_endpoint::InputTokenCountEndpoint;
pub use semantic::SemanticApiEndpoint;
pub use sse::AnthropicMessagesSseDecoder;
pub use sse::OpenAiChatCompletionsSseDecoder;
pub use sse::OpenAiResponsesSseDecoder;
pub use token_count::InputTokenCount;
pub use zeta_protocol::ContentPart;
pub use zeta_protocol::ImageDetail;
pub use zeta_protocol::InputItem;
pub use zeta_protocol::Message;
pub use zeta_protocol::MessageRole;
pub use zeta_protocol::ModelRequest;
pub use zeta_protocol::ModelResponse;
pub use zeta_protocol::ModelStreamEvent;
pub use zeta_protocol::ModelUsage;
pub use zeta_protocol::ReasoningConfig;
pub use zeta_protocol::ReasoningEffort;
pub use zeta_protocol::ResponseItem as OutputItem;
pub use zeta_protocol::StopReason;
pub use zeta_protocol::ToolCall;
pub use zeta_protocol::ToolCallId;
pub use zeta_protocol::ToolChoice;
pub use zeta_protocol::ToolDefinition;
pub use zeta_protocol::ToolName;
pub use zeta_protocol::ToolResult;
