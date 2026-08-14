//! Zeta-owned local Marketplace package lifecycle.
//!
//! This crate owns the product-local artifact store, installations, update/uninstall state,
//! capability leases, and opaque resource access. Signed discovery and downloads arrive through
//! an injected remote registry client.

mod activation;
mod manager;
mod store;

pub use manager::LocalCapabilitySource;
pub use manager::MarketplaceManager;

#[cfg(test)]
#[path = "manager_tests.rs"]
mod tests;
