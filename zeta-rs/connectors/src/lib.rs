//! Connector identity, account lifecycle, runtime binding, and immutable snapshots.
//!
//! This crate is backend-neutral. It does not discover Plugin packages, perform OAuth, store
//! credential bytes, start MCP sessions, or project Connector state into a product protocol.

mod connection;
mod definition;
mod error;
mod identity;
mod snapshot;

pub use connection::ConnectorAccount;
pub use connection::ConnectorConnection;
pub use connection::ConnectorConnectionState;
pub use connection::ConnectorConnectionUpdate;
pub use definition::ConnectorDefinition;
pub use definition::ConnectorDefinitionDigest;
pub use definition::ConnectorRuntimeBinding;
pub use error::ConnectorError;
pub use error::ConnectorErrorKind;
pub use identity::ConnectorAccountId;
pub use identity::ConnectorConnectionGeneration;
pub use identity::ConnectorCredentialRef;
pub use identity::ConnectorId;
pub use identity::ConnectorSnapshotGeneration;
pub use snapshot::ConnectorEntry;
pub use snapshot::ConnectorSnapshot;

#[cfg(test)]
#[path = "connector_tests.rs"]
mod tests;
