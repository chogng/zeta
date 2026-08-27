//! Zeta's OpenTelemetry integration boundary.
//!
//! Production provider and exporter wiring is intentionally not installed yet. The isolated
//! in-memory implementation is available only through the non-default `mock` feature.

#[cfg(feature = "mock")]
pub mod mock;
