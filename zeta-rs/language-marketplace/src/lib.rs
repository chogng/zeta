//! TUF-backed discovery and installation of signed Marketplace language-server packages.
//!
//! This crate owns remote metadata refresh, exact package identity, compatibility evaluation,
//! bounded archive extraction, Marketplace package-digest verification, and handoff to the
//! durable language-server activation authority. Provider construction and LSP execution remain
//! outside this crate.

mod archive;
mod error;
mod model;
mod remote;
mod transport;

pub use error::LanguageMarketplaceError;
pub use error::LanguageMarketplaceErrorKind;
pub use model::LanguageMarketplaceCompatibility;
pub use model::LanguageMarketplaceEntry;
pub use model::LanguageMarketplaceId;
pub use model::LanguageMarketplaceRuntime;
pub use model::LanguagePackageDigest;
pub use model::LanguagePackageId;
pub use model::LanguagePackageVersion;
pub use remote::RemoteLanguageMarketplace;
pub use remote::RemoteLanguageMarketplaceConfig;
pub use remote::RemoteLanguageMarketplaceSnapshot;

#[cfg(test)]
#[path = "archive_tests.rs"]
mod archive_tests;

#[cfg(test)]
#[path = "model_tests.rs"]
mod model_tests;

#[cfg(test)]
#[path = "remote_distribution_tests.rs"]
mod remote_distribution_tests;
