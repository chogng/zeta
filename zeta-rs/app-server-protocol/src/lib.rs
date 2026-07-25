//! Versioned external RPC DTOs. Domain entities deliberately remain private to `zeta-core`.

pub mod common;
mod schema;
pub mod v1;

pub const CURRENT_PROTOCOL_VERSION: u32 = 1;

pub use schema::json_schema_v1;
pub use schema::schema_hash_v1;
pub use schema::typescript_v1;
