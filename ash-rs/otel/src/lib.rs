//! Production OpenTelemetry provider and a bounded local diagnostic exporter.

mod provider;
pub use provider::Telemetry;

#[cfg(feature = "mock")]
pub mod mock;
