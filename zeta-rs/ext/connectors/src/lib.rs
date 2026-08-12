//! Connector identity, connection lifecycle, and catalog discovery projection.

mod identity;
mod projection;

pub use identity::ConnectedAccount;
pub use identity::ConnectorBinding;
pub use identity::ConnectorConnectionState;
pub use identity::ConnectorId;
pub use identity::ConnectorIdentityError;
pub use projection::ConnectorCatalog;
pub use projection::ConnectorCatalogError;
pub use projection::ConnectorEntry;

#[cfg(test)]
#[path = "projection_tests.rs"]
mod tests;
