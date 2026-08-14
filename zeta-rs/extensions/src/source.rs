use std::path::PathBuf;

/// Identifies the provenance of a static extension package.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExtensionRootKind {
    /// A package shipped with the product installation.
    BuiltIn,
    /// A package selected by the effective Plugin authority snapshot.
    Plugin,
    /// A package installed by the local Marketplace Manager.
    Marketplace,
    /// A package installed below the user's trusted profile extension directory.
    User,
}

/// Filesystem root containing direct-child extension packages.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtensionRoot {
    /// Provenance reported in catalog diagnostics and descriptors.
    pub kind: ExtensionRootKind,
    /// Root path supplied by the host composition root.
    pub path: PathBuf,
}

impl ExtensionRoot {
    /// Creates a built-in extension root.
    pub fn built_in(path: impl Into<PathBuf>) -> Self {
        Self {
            kind: ExtensionRootKind::BuiltIn,
            path: path.into(),
        }
    }

    /// Creates a user extension root.
    pub fn user(path: impl Into<PathBuf>) -> Self {
        Self {
            kind: ExtensionRootKind::User,
            path: path.into(),
        }
    }
}

/// One exact declarative extension package selected by an external authority snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DynamicExtensionPackageSource {
    /// Stable authority-owned label used in diagnostics.
    pub subject: String,
    /// Exact package directory containing `package.json`.
    pub path: PathBuf,
    /// Verified package provenance projected into the catalog.
    pub kind: ExtensionRootKind,
}

impl DynamicExtensionPackageSource {
    /// Creates a Plugin-authorized exact package source.
    pub fn plugin(subject: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            subject: subject.into(),
            path: path.into(),
            kind: ExtensionRootKind::Plugin,
        }
    }

    /// Creates a Marketplace-authorized exact package source.
    pub fn marketplace(subject: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            subject: subject.into(),
            path: path.into(),
            kind: ExtensionRootKind::Marketplace,
        }
    }
}

/// Immutable exact extension package set published by a dynamic authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DynamicExtensionSourceSnapshot {
    /// Monotonically changing authority generation.
    pub generation: u64,
    /// Exact package directories in stable authority order.
    pub packages: Vec<DynamicExtensionPackageSource>,
}

/// Supplies exact extension package directories without transferring lifecycle ownership.
///
/// Implementations must return only immutable paths selected by their own live authority. The
/// catalog revalidates and freezes every package before exposing descriptors or resource bytes.
pub trait DynamicExtensionSourceProvider: Send + Sync {
    /// Returns the current authority generation and exact package sources.
    fn snapshot(&self) -> Result<DynamicExtensionSourceSnapshot, String>;
}
