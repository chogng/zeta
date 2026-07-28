//! Structured LLM-assisted review of fully resolved Agent actions.
//!
//! Classifier output is advisory. This crate never grants capabilities, starts actions, or
//! interprets a recommendation as authorization to bypass a sandbox.

mod classifier;

pub use classifier::{AutoReviewError, LlmActionClassifier, ReviewModel, ReviewModelRequest};
