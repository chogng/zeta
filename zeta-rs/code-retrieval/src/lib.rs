//! Policy-neutral orchestration of local lexical, local semantic, and optional remote recall.
//!
//! This crate owns candidate fan-out, reciprocal-rank fusion, identity deduplication, current
//! source verification, graceful source fallback, and content budgets. Lexical indexing remains in
//! `zeta-code-index`, local dense recall in `zeta-code-index-semantic`, and grant and
//! remote-generation authority in `zeta-code-index-cloud`.

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
