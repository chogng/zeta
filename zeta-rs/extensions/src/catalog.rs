use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::diagnostic::diagnostic;
use crate::resource::is_within;
use crate::resource::mime_type;
use crate::resource::validate_relative_path;

const MAX_MANIFEST_BYTES: u64 = 4 * 1024 * 1024;
const MAX_EXTENSION_RESOURCE_BYTES: usize = 16 * 1024 * 1024;
const MAX_EXTENSION_ID_LENGTH: usize = 160;
const MAX_MANIFEST_FIELD_LENGTH: usize = 256;

/// Selects whether a catalog query may reuse its previous discovery snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExtensionCatalogReload {
    /// Return the previous snapshot, scanning only when no snapshot exists yet.
    Cached,
    /// Rescan all configured roots and publish a new generation.
    Refresh,
}

/// Identifies the provenance of a static extension package.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExtensionRootKind {
    /// A package shipped with the product installation.
    BuiltIn,
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

/// Static metadata exposed to a host after manifest identity validation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtensionDescriptor {
    /// Canonical `publisher.name` extension identity.
    pub id: String,
    /// Manifest `name` component.
    pub name: String,
    /// Manifest `publisher` component.
    pub publisher: String,
    /// Manifest `version` value.
    pub version: String,
    /// Validated display name, defaulting to `name` when omitted.
    pub display_name: String,
    /// Trusted root provenance.
    pub source_kind: ExtensionSourceKind,
    /// Canonical JSON representation of the complete manifest.
    pub manifest_json: String,
    /// SHA-256 digest of the manifest bytes as stored on disk.
    pub manifest_sha256: String,
}

/// Provenance attached to a discovered extension descriptor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExtensionSourceKind {
    /// The package came from the product installation.
    BuiltIn,
    /// The package came from the user's profile extension directory.
    User,
}

/// Stable diagnostic category for a package discovery failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExtensionDiagnosticCode {
    /// A configured root could not be read.
    SourceUnavailable,
    /// The package manifest is absent or malformed.
    InvalidManifest,
    /// A later root attempted to register an already-used extension ID.
    DuplicateExtension,
    /// A package or manifest path escaped its trusted root.
    PathEscapesRoot,
    /// A requested package resource does not exist or is not a file.
    ResourceNotFound,
    /// A manifest or resource exceeded its configured size limit.
    ResourceTooLarge,
}

/// Diagnostic emitted while scanning an extension root.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtensionDiagnostic {
    /// Human-readable root provenance label.
    pub source: String,
    /// Root child or other subject associated with the diagnostic.
    pub subject: Option<String>,
    /// Stable diagnostic category.
    pub code: ExtensionDiagnosticCode,
    /// Diagnostic message suitable for logs and host presentation.
    pub message: String,
}

/// Immutable result of one extension catalog generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtensionCatalogSnapshot {
    /// Monotonically increasing scan generation.
    pub generation: u64,
    /// Successfully discovered extensions in deterministic order.
    pub extensions: Vec<ExtensionDescriptor>,
    /// Non-fatal discovery diagnostics in deterministic scan order.
    pub diagnostics: Vec<ExtensionDiagnostic>,
}

/// Resource bytes read from a discovered extension package.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtensionResource {
    /// MIME type inferred from the resource extension.
    pub mime_type: String,
    /// Bounded resource bytes.
    pub bytes: Vec<u8>,
}

/// Failure returned when a host requests an extension or package resource.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExtensionCatalogError {
    /// The extension ID is not in the latest discovered catalog.
    NotFound,
    /// The requested relative path is unsafe.
    InvalidPath,
    /// The requested resource is absent or not a regular file.
    ResourceNotFound,
    /// The resource exceeds the shared static resource limit.
    ResourceTooLarge,
    /// The host filesystem operation failed for another reason.
    OperationFailed,
}

#[derive(Default)]
pub struct ExtensionCatalog {
    roots: Vec<ExtensionRoot>,
    generation: u64,
    snapshot: Option<ExtensionCatalogSnapshot>,
    discovered: BTreeMap<String, DiscoveredExtension>,
}

struct DiscoveredExtension {
    package_root: PathBuf,
    descriptor: ExtensionDescriptor,
}

impl ExtensionCatalog {
    /// Creates a catalog from trusted built-in and user roots.
    pub fn new(roots: Vec<ExtensionRoot>) -> Self {
        Self {
            roots,
            ..Self::default()
        }
    }

    /// Reports whether at least one host-configured root exists.
    pub fn is_available(&self) -> bool {
        !self.roots.is_empty()
    }

    /// Lists extensions, rescanning roots when requested or when no snapshot exists.
    pub fn list(&mut self, reload: ExtensionCatalogReload) -> ExtensionCatalogSnapshot {
        if reload == ExtensionCatalogReload::Cached {
            if let Some(snapshot) = &self.snapshot {
                return snapshot.clone();
            }
        }
        let (extensions, diagnostics, discovered) = self.scan();
        self.generation = self.generation.saturating_add(1);
        self.discovered = discovered;
        let snapshot = ExtensionCatalogSnapshot {
            generation: self.generation,
            extensions,
            diagnostics,
        };
        self.snapshot = Some(snapshot.clone());
        snapshot
    }

    /// Opens one bounded resource below a discovered package root.
    pub fn open_resource(
        &mut self,
        extension_id: &str,
        path: &str,
    ) -> Result<ExtensionResource, ExtensionCatalogError> {
        if self.snapshot.is_none() {
            let _ = self.list(ExtensionCatalogReload::Cached);
        }
        let extension = self
            .discovered
            .get(extension_id)
            .ok_or(ExtensionCatalogError::NotFound)?;
        let relative_path = validate_relative_path(path)?;
        let candidate = extension.package_root.join(&relative_path);
        let canonical = fs::canonicalize(&candidate).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                ExtensionCatalogError::ResourceNotFound
            } else {
                ExtensionCatalogError::OperationFailed
            }
        })?;
        if !is_within(&extension.package_root, &canonical) {
            return Err(ExtensionCatalogError::InvalidPath);
        }
        let metadata =
            fs::metadata(&canonical).map_err(|_| ExtensionCatalogError::ResourceNotFound)?;
        if !metadata.is_file() {
            return Err(ExtensionCatalogError::ResourceNotFound);
        }
        if metadata.len() > MAX_EXTENSION_RESOURCE_BYTES as u64 {
            return Err(ExtensionCatalogError::ResourceTooLarge);
        }
        let bytes = fs::read(&canonical).map_err(|_| ExtensionCatalogError::OperationFailed)?;
        if bytes.len() > MAX_EXTENSION_RESOURCE_BYTES {
            return Err(ExtensionCatalogError::ResourceTooLarge);
        }
        Ok(ExtensionResource {
            mime_type: mime_type(&canonical),
            bytes,
        })
    }

    fn scan(
        &self,
    ) -> (
        Vec<ExtensionDescriptor>,
        Vec<ExtensionDiagnostic>,
        BTreeMap<String, DiscoveredExtension>,
    ) {
        let mut extensions = Vec::new();
        let mut diagnostics = Vec::new();
        let mut discovered = BTreeMap::new();
        for root in &self.roots {
            scan_root(root, &mut extensions, &mut diagnostics, &mut discovered);
        }
        (extensions, diagnostics, discovered)
    }
}

fn scan_root(
    root: &ExtensionRoot,
    extensions: &mut Vec<ExtensionDescriptor>,
    diagnostics: &mut Vec<ExtensionDiagnostic>,
    discovered: &mut BTreeMap<String, DiscoveredExtension>,
) {
    let canonical_root = match fs::canonicalize(&root.path) {
        Ok(path) => path,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(_) => {
            diagnostics.push(diagnostic(
                root,
                None,
                ExtensionDiagnosticCode::SourceUnavailable,
                "extension root is unavailable",
            ));
            return;
        }
    };
    let entries = match fs::read_dir(&canonical_root) {
        Ok(entries) => entries,
        Err(_) => {
            diagnostics.push(diagnostic(
                root,
                None,
                ExtensionDiagnosticCode::SourceUnavailable,
                "extension root cannot be read",
            ));
            return;
        }
    };
    let mut packages = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let is_directory = entry.file_type().ok()?.is_dir();
            is_directory.then_some((entry.file_name().to_string_lossy().into_owned(), path))
        })
        .collect::<Vec<_>>();
    packages.sort_by(|left, right| left.0.cmp(&right.0));
    for (subject, package_path) in packages {
        match discover_package(root, &canonical_root, &package_path) {
            Ok(extension) => {
                if discovered.contains_key(&extension.descriptor.id) {
                    diagnostics.push(diagnostic(
                        root,
                        Some(subject),
                        ExtensionDiagnosticCode::DuplicateExtension,
                        "extension ID is already registered",
                    ));
                    continue;
                }
                extensions.push(extension.descriptor.clone());
                discovered.insert(extension.descriptor.id.clone(), extension);
            }
            Err((code, message)) => {
                diagnostics.push(diagnostic(root, Some(subject), code, message))
            }
        }
    }
}

fn discover_package(
    root: &ExtensionRoot,
    canonical_root: &Path,
    package_path: &Path,
) -> Result<DiscoveredExtension, (ExtensionDiagnosticCode, &'static str)> {
    let package_root = fs::canonicalize(package_path).map_err(|_| {
        (
            ExtensionDiagnosticCode::InvalidManifest,
            "extension package cannot be resolved",
        )
    })?;
    if !is_within(canonical_root, &package_root) {
        return Err((
            ExtensionDiagnosticCode::PathEscapesRoot,
            "extension package escapes its source root",
        ));
    }
    let manifest_path = package_root.join("package.json");
    let canonical_manifest = match fs::canonicalize(&manifest_path) {
        Ok(path) => path,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err((
                ExtensionDiagnosticCode::InvalidManifest,
                "extension package.json is missing",
            ));
        }
        Err(_) => {
            return Err((
                ExtensionDiagnosticCode::PathEscapesRoot,
                "extension package.json cannot be resolved inside its package root",
            ));
        }
    };
    if !is_within(&package_root, &canonical_manifest) {
        return Err((
            ExtensionDiagnosticCode::PathEscapesRoot,
            "extension package.json escapes its package root",
        ));
    }
    let manifest_metadata = fs::metadata(&canonical_manifest).map_err(|_| {
        (
            ExtensionDiagnosticCode::InvalidManifest,
            "extension package.json cannot be read",
        )
    })?;
    if manifest_metadata.len() > MAX_MANIFEST_BYTES {
        return Err((
            ExtensionDiagnosticCode::InvalidManifest,
            "extension package.json is too large",
        ));
    }
    let manifest_bytes = fs::read(&canonical_manifest).map_err(|_| {
        (
            ExtensionDiagnosticCode::InvalidManifest,
            "extension package.json cannot be read",
        )
    })?;
    let manifest_text = String::from_utf8(manifest_bytes.clone()).map_err(|_| {
        (
            ExtensionDiagnosticCode::InvalidManifest,
            "extension package.json must be UTF-8",
        )
    })?;
    let manifest = serde_json::from_str::<Value>(&manifest_text).map_err(|_| {
        (
            ExtensionDiagnosticCode::InvalidManifest,
            "extension package.json is not valid JSON",
        )
    })?;
    let object = manifest.as_object().ok_or((
        ExtensionDiagnosticCode::InvalidManifest,
        "extension package.json must contain an object",
    ))?;
    let name = required_manifest_string(object, "name")?;
    let publisher = required_manifest_string(object, "publisher")?;
    let version = required_manifest_string(object, "version")?;
    let display_name = match object.get("displayName") {
        None => name.clone(),
        Some(Value::String(value))
            if !value.trim().is_empty() && value.len() <= MAX_MANIFEST_FIELD_LENGTH =>
        {
            value.clone()
        }
        Some(_) => {
            return Err((
                ExtensionDiagnosticCode::InvalidManifest,
                "extension display name is invalid",
            ));
        }
    };
    if !is_manifest_component(&name) || !is_manifest_component(&publisher) {
        return Err((
            ExtensionDiagnosticCode::InvalidManifest,
            "extension name and publisher are invalid",
        ));
    }
    let id = format!("{publisher}.{name}");
    if id.len() > MAX_EXTENSION_ID_LENGTH {
        return Err((
            ExtensionDiagnosticCode::InvalidManifest,
            "extension ID is too long",
        ));
    }
    let canonical_manifest = serde_json::to_string(&manifest).map_err(|_| {
        (
            ExtensionDiagnosticCode::InvalidManifest,
            "extension package.json cannot be serialized",
        )
    })?;
    let manifest_sha256 = format!("sha256:{:x}", Sha256::digest(&manifest_bytes));
    Ok(DiscoveredExtension {
        package_root,
        descriptor: ExtensionDescriptor {
            id,
            name,
            publisher,
            version,
            display_name,
            source_kind: source_kind(&root.kind),
            manifest_json: canonical_manifest,
            manifest_sha256,
        },
    })
}

fn required_manifest_string(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<String, (ExtensionDiagnosticCode, &'static str)> {
    let value = object
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or((
            ExtensionDiagnosticCode::InvalidManifest,
            "extension manifest has a missing required string",
        ))?;
    if value.len() > MAX_MANIFEST_FIELD_LENGTH {
        return Err((
            ExtensionDiagnosticCode::InvalidManifest,
            "extension manifest field is too long",
        ));
    }
    Ok(value.to_string())
}

fn is_manifest_component(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn source_kind(kind: &ExtensionRootKind) -> ExtensionSourceKind {
    match kind {
        ExtensionRootKind::BuiltIn => ExtensionSourceKind::BuiltIn,
        ExtensionRootKind::User => ExtensionSourceKind::User,
    }
}
