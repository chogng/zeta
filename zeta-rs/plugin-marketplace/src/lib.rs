//! Verified remote distribution for product-managed Plugin Marketplaces.
//!
//! This crate owns TUF metadata refresh, delegated publisher verification, verified target
//! download, bounded archive materialization, and immutable local cache snapshots. Plugin
//! installation and activation authority remain owned by `zeta-plugins`.

mod archive;
mod error;
mod metadata;
mod remote;
mod transport;

pub use error::RemoteMarketplaceError;
pub use error::RemoteMarketplaceErrorKind;
pub use remote::RemotePluginMarketplace;
pub use remote::RemotePluginMarketplaceConfig;
pub use remote::RemotePluginMarketplaceSnapshot;

#[cfg(test)]
#[path = "archive_tests.rs"]
mod archive_tests;

#[cfg(test)]
#[path = "metadata_tests.rs"]
mod metadata_tests;

#[cfg(test)]
#[path = "remote_distribution_tests.rs"]
mod remote_distribution_tests;
