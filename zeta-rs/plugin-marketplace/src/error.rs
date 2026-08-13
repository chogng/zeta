use std::error::Error;
use std::fmt;

/// Stable failure category for remote Marketplace refresh and materialization.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RemoteMarketplaceErrorKind {
    InvalidConfiguration,
    MetadataUntrusted,
    DistributionUnavailable,
    PackageUnsafe,
    CacheUnavailable,
}

/// Sanitized remote Marketplace failure.
///
/// Messages never include response bodies, trusted metadata contents, URLs, or canonical host
/// paths.
#[derive(Debug)]
pub struct RemoteMarketplaceError {
    kind: RemoteMarketplaceErrorKind,
    message: String,
}

impl RemoteMarketplaceError {
    pub(crate) fn new(
        kind: RemoteMarketplaceErrorKind,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn kind(&self) -> RemoteMarketplaceErrorKind {
        self.kind
    }
}

impl fmt::Display for RemoteMarketplaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for RemoteMarketplaceError {}
