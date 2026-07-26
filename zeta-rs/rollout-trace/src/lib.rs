//! Read-only, serializable traces of one Session's authoritative rollout.
//!
//! A trace preserves the independent Session and Thread sequences from the durable source. It is
//! an inspection artifact, never a second authority or an input to runtime decisions. Raw durable
//! events can contain sensitive user and tool data, so this crate deliberately performs no I/O or
//! implicit export.

mod error;
mod trace;

pub use error::RolloutTraceError;
pub use trace::ROLLOUT_TRACE_FORMAT_VERSION;
pub use trace::RolloutTrace;
pub use trace::ThreadRolloutTrace;
pub use trace::capture_session_trace;

#[cfg(test)]
#[path = "trace_tests.rs"]
mod tests;
