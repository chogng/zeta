//! Codex-compatible ChatGPT sign-in, credential reuse, and authentication maintenance.

mod credential;
mod device_flow;
mod maintenance;
mod oauth;
mod storage;

pub use maintenance::ChatGptAuthManagement;

pub use storage::codex_home;

pub use oauth::CHATGPT_RESPONSES_BASE_URL;
pub use oauth::ChatGptError;
pub use oauth::ChatGptOAuth;
pub use oauth::OPENAI_CHATGPT_PROVIDER_ID;

#[cfg(test)]
#[path = "chatgpt_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "live_tests.rs"]
mod live_tests;
