//! Provider-independent model catalog discovery, caching, merge, query, and resolution.

mod cache;
mod error;
mod filter;
mod manager;
mod merge;
mod policy;
mod scope;
mod snapshot;
mod source;

pub use error::ModelsManagerError;
pub use filter::AvailabilityFilter;
pub use filter::CatalogQuery;
pub use filter::ModelCapability;
pub use filter::ModelRequirements;
pub use filter::UnknownCapabilityPolicy;
pub use manager::CatalogReadSource;
pub use manager::ModelsManager;
pub use policy::CatalogFreshnessPolicy;
pub use policy::CatalogReadPolicy;
pub use scope::CatalogScopeKey;
pub use scope::CatalogSourceScopeId;
pub use scope::InvalidCatalogScope;
pub use snapshot::CatalogGeneration;
pub use snapshot::CatalogWarning;
pub use snapshot::CatalogWarningCode;
pub use snapshot::MetadataSource;
pub use snapshot::ModelCapabilitiesProvenance;
pub use snapshot::ModelCatalogEntry;
pub use snapshot::ModelCatalogSnapshot;
pub use snapshot::ModelMetadataProvenance;
pub use snapshot::ResolvedModel;
pub use source::CatalogCacheHint;
pub use source::CatalogDiscoveryOutcome;
pub use source::CatalogDiscoveryRequest;
pub use source::CatalogNotModified;
pub use source::CatalogSourceError;
pub use source::CatalogSourceErrorKind;
pub use source::CatalogSourceFuture;
pub use source::CatalogValidator;
pub use source::DiscoveredCatalog;
pub use source::DiscoveredModel;
pub use source::DiscoveryCoverage;
pub use source::ModelCapabilitiesPatch;
pub use source::ModelCatalogSource;
pub use source::ModelMetadataPatch;

pub use zeta_protocol::ModelAvailability;
pub use zeta_protocol::ModelCatalogFreshness;
pub use zeta_protocol::ModelLifecycle;
pub use zeta_protocol::ModelMetadataQuality;

#[cfg(test)]
#[path = "manager_tests.rs"]
mod tests;
