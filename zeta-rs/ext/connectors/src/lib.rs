//! Plugin contribution and product discovery integration for Connector snapshots.

mod projection;

pub use projection::ConnectorCatalog;
pub use projection::ConnectorCatalogError;

#[cfg(test)]
#[path = "projection_tests.rs"]
mod tests;
