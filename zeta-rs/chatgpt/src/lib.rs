//! Native ChatGPT subscription OAuth and authenticated Responses API targets.

mod credential;
mod device_flow;
mod oauth;

pub use oauth::CHATGPT_RESPONSES_BASE_URL;
pub use oauth::ChatGptError;
pub use oauth::ChatGptOAuth;
pub use oauth::OPENAI_CHATGPT_PROVIDER_ID;

#[cfg(test)]
#[path = "chatgpt_tests.rs"]
mod tests;
