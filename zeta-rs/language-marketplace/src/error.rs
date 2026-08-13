/// Stable failure category exposed at the App Server boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LanguageMarketplaceErrorKind {
    InvalidConfiguration,
    MetadataUntrusted,
    DistributionUnavailable,
    PackageUnsafe,
    CacheUnavailable,
    Incompatible,
    ActivationUnavailable,
}

/// Sanitized Language Marketplace failure that does not expose remote response bodies or paths.
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct LanguageMarketplaceError {
    kind: LanguageMarketplaceErrorKind,
    message: &'static str,
}

impl LanguageMarketplaceError {
    pub(crate) const fn new(kind: LanguageMarketplaceErrorKind, message: &'static str) -> Self {
        Self { kind, message }
    }

    /// Returns the stable failure category used by protocol error mapping.
    pub const fn kind(&self) -> LanguageMarketplaceErrorKind {
        self.kind
    }
}
