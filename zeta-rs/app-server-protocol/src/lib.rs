//! External RPC protocol, wire envelopes, and generated contract artifacts.
//!
//! Domain entities deliberately remain private to `zeta-core`.

mod export;
mod listen_info;
pub mod protocol;
pub mod rpc;
mod typescript_decoder;

pub use export::JSON_SCHEMA_FIXTURE;
pub use export::TYPESCRIPT_FILES;
pub use export::TYPESCRIPT_FIXTURE_DIRECTORY;
pub use export::json_schema;
pub use export::schema_hash;
pub use export::typescript;
pub use export::typescript_files;
pub use listen_info::AppServerListenInfo;
pub use listen_info::AppServerListenInfoError;

#[cfg(test)]
#[path = "schema_fixtures.rs"]
mod tests;

#[cfg(test)]
#[path = "protocol_compatibility_tests.rs"]
mod protocol_compatibility_tests;
