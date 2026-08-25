use std::error::Error;
use std::fmt;

/// Stable failure returned by an operating-system service capability.
#[derive(Debug)]
pub struct SystemServiceError {
    service: &'static str,
    kind: SystemServiceErrorKind,
}

#[derive(Debug)]
enum SystemServiceErrorKind {
    Unsupported,
    Backend(Box<dyn Error + Send + Sync>),
}

impl SystemServiceError {
    /// Creates a stable failure for a capability unavailable on the current platform.
    pub const fn unsupported(service: &'static str) -> Self {
        Self {
            service,
            kind: SystemServiceErrorKind::Unsupported,
        }
    }

    pub(crate) fn backend(
        service: &'static str,
        source: impl Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            service,
            kind: SystemServiceErrorKind::Backend(Box::new(source)),
        }
    }

    /// Returns whether the current platform has no implementation for this capability.
    pub const fn is_unsupported(&self) -> bool {
        matches!(self.kind, SystemServiceErrorKind::Unsupported)
    }
}

impl fmt::Display for SystemServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            SystemServiceErrorKind::Unsupported => {
                write!(
                    formatter,
                    "{} is unsupported on this platform",
                    self.service
                )
            }
            SystemServiceErrorKind::Backend(source) => {
                write!(formatter, "{} failed: {source}", self.service)
            }
        }
    }
}

impl Error for SystemServiceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.kind {
            SystemServiceErrorKind::Unsupported => None,
            SystemServiceErrorKind::Backend(source) => Some(source.as_ref()),
        }
    }
}
