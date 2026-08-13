//! Local Shell-versus-Agent input classification with pinned embedded assets.

mod classifier;
mod rules;

pub use classifier::InputClassification;
pub use classifier::InputClassificationSource;
pub use classifier::InputRoute;
pub use classifier::MODEL_SHA256;
pub use classifier::MODEL_VERSION;
pub use classifier::ShellCommandEvidence;
pub use classifier::TOKENIZER_SHA256;
pub use classifier::classify_input;
pub use classifier::start_background_warmup;

#[cfg(test)]
#[path = "input_classifier_tests.rs"]
mod tests;
