//! Policy-neutral orchestration of local and cloud code-index recall.
//!
//! This crate owns candidate fan-out, reciprocal-rank fusion, identity deduplication, current
//! source verification, graceful cloud fallback, and content budgets. Local indexing remains in
//! `zeta-code-index`; grant and remote-generation authority remain in `zeta-code-index-cloud`.

mod error;
mod service;
mod types;

pub use error::CodeRetrievalError;
pub use service::CodeRetrievalService;
pub use types::CodeRetrievalBudget;
pub use types::CodeRetrievalDegradation;
pub use types::CodeRetrievalHit;
pub use types::CodeRetrievalOrigin;
pub use types::CodeRetrievalQuery;
pub use types::CodeRetrievalResult;

#[cfg(test)]
#[path = "retrieval_tests.rs"]
mod tests;
