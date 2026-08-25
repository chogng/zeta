//! External RPC protocol, wire envelopes, and generated contract artifacts.
//!
//! Domain entities deliberately remain private to `zeta-core`.

mod export;
pub mod protocol;
pub mod rpc;

pub use export::{JSON_SCHEMA_FIXTURE, TYPESCRIPT_FIXTURE, json_schema, schema_hash, typescript};

#[cfg(test)]
#[path = "schema_fixtures.rs"]
mod tests;

#[cfg(test)]
#[path = "protocol_compatibility_tests.rs"]
mod protocol_compatibility_tests;
