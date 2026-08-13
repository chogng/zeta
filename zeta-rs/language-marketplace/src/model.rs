use std::collections::BTreeMap;
use std::fmt;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use semver::Version;
use semver::VersionReq;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;

use crate::LanguageMarketplaceError;
use crate::LanguageMarketplaceErrorKind;

const MAX_PACKAGE_ID_BYTES: usize = 128;
const MAX_LOCAL_ID_BYTES: usize = 64;
const MAX_PATH_BYTES: usize = 512;
const SHA256_PREFIX: &str = "sha256:";

/// Validated product-configured identity of one remote Marketplace source.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LanguageMarketplaceId(String);

impl LanguageMarketplaceId {
    pub fn new(value: impl Into<String>) -> Result<Self, LanguageMarketplaceError> {
        let value = value.into();
        if !valid_identifier_segment(&value) {
            return Err(metadata_error());
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Stable publisher-qualified Marketplace package identity.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LanguagePackageId(String);

impl LanguagePackageId {
    pub fn new(value: impl Into<String>) -> Result<Self, LanguageMarketplaceError> {
        let value = value.into();
        let valid = value.len() <= MAX_PACKAGE_ID_BYTES
            && value.matches('/').count() == 1
            && value.split_once('/').is_some_and(|(publisher, name)| {
                valid_identifier_segment(publisher) && valid_identifier_segment(name)
            });
        if !valid {
            return Err(metadata_error());
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn publisher(&self) -> &str {
        self.0
            .split_once('/')
            .expect("validated package ID has one separator")
            .0
    }
}

impl fmt::Display for LanguagePackageId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for LanguagePackageId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for LanguagePackageId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

/// Exact semantic version of a Marketplace language package.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LanguagePackageVersion(Version);

impl LanguagePackageVersion {
    pub fn new(value: impl AsRef<str>) -> Result<Self, LanguageMarketplaceError> {
        Version::parse(value.as_ref())
            .map(Self)
            .map_err(|_| metadata_error())
    }
}

impl fmt::Display for LanguagePackageVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl Serialize for LanguagePackageVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for LanguagePackageVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

/// Lowercase self-describing digest of one normalized Marketplace package tree.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LanguagePackageDigest(String);

impl LanguagePackageDigest {
    pub fn new(value: impl Into<String>) -> Result<Self, LanguageMarketplaceError> {
        let value = value.into();
        let valid = value.strip_prefix(SHA256_PREFIX).is_some_and(|hex| {
            hex.len() == 64
                && hex
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        });
        if !valid {
            return Err(metadata_error());
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for LanguagePackageDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for LanguagePackageDigest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for LanguagePackageDigest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

/// Runtime contract declared by the signed executable capability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LanguageMarketplaceRuntime {
    Node,
    Direct,
    /// Schema v1 packages predate an explicit runtime field; the product adapter must recognize
    /// the server identity before considering this compatible.
    LegacyUnspecified,
}

/// Product-version compatibility result computed entirely from signed catalog metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LanguageMarketplaceCompatibility {
    Compatible,
    Incompatible(String),
}

impl LanguageMarketplaceCompatibility {
    pub const fn is_compatible(&self) -> bool {
        matches!(self, Self::Compatible)
    }

    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Compatible => None,
            Self::Incompatible(reason) => Some(reason),
        }
    }
}

/// One signed, exact and installable language-server catalog entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageMarketplaceEntry {
    pub(crate) marketplace_id: LanguageMarketplaceId,
    pub(crate) package_id: LanguagePackageId,
    pub(crate) version: LanguagePackageVersion,
    pub(crate) digest: LanguagePackageDigest,
    pub(crate) display_name: String,
    pub(crate) description: String,
    pub(crate) license: String,
    pub(crate) server_id: String,
    pub(crate) executable_path: PathBuf,
    pub(crate) runtime: LanguageMarketplaceRuntime,
    pub(crate) languages: Vec<String>,
    pub(crate) file_extensions: Vec<String>,
    pub(crate) compatibility: LanguageMarketplaceCompatibility,
    pub(crate) target_name: String,
    pub(crate) target_length: u64,
    pub(crate) package_file_count: u64,
    pub(crate) package_size_bytes: u64,
}

impl LanguageMarketplaceEntry {
    pub fn marketplace_id(&self) -> &LanguageMarketplaceId {
        &self.marketplace_id
    }

    pub fn package_id(&self) -> &LanguagePackageId {
        &self.package_id
    }

    pub fn version(&self) -> &LanguagePackageVersion {
        &self.version
    }

    pub fn digest(&self) -> &LanguagePackageDigest {
        &self.digest
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn license(&self) -> &str {
        &self.license
    }

    pub fn server_id(&self) -> &str {
        &self.server_id
    }

    pub fn runtime(&self) -> LanguageMarketplaceRuntime {
        self.runtime
    }

    pub fn languages(&self) -> &[String] {
        &self.languages
    }

    pub fn file_extensions(&self) -> &[String] {
        &self.file_extensions
    }

    pub fn compatibility(&self) -> &LanguageMarketplaceCompatibility {
        &self.compatibility
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PackageTargetMetadata {
    pub(crate) schema_version: u32,
    pub(crate) id: LanguagePackageId,
    pub(crate) version: LanguagePackageVersion,
    pub(crate) package_digest: LanguagePackageDigest,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PackageCatalogMetadata {
    pub(crate) schema_version: u32,
    pub(crate) manifest: PackageManifest,
    #[serde(rename = "consumerMetadata", default)]
    pub(crate) _consumer_metadata: BTreeMap<String, serde_json::Value>,
    pub(crate) package_file_count: u64,
    pub(crate) package_size_bytes: u64,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PackageManifest {
    pub(crate) schema_version: u32,
    pub(crate) package_type: String,
    pub(crate) source: String,
    pub(crate) id: LanguagePackageId,
    pub(crate) version: LanguagePackageVersion,
    pub(crate) display_name: String,
    pub(crate) description: String,
    pub(crate) license: String,
    pub(crate) languages: Vec<LanguageDeclaration>,
    pub(crate) capabilities: Vec<CapabilityDeclaration>,
    #[serde(default)]
    pub(crate) consumers: BTreeMap<String, ConsumerDeclaration>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LanguageDeclaration {
    pub(crate) id: String,
    pub(crate) display_name: String,
    #[serde(default)]
    pub(crate) aliases: Vec<String>,
    #[serde(default)]
    pub(crate) file_extensions: Vec<String>,
    #[serde(default)]
    pub(crate) lsp: bool,
    #[serde(default)]
    pub(crate) language_server: Option<String>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CapabilityDeclaration {
    pub(crate) kind: String,
    pub(crate) id: String,
    pub(crate) path: String,
    #[serde(default)]
    pub(crate) runtime: Option<ExecutableRuntimeDocument>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ExecutableRuntimeDocument {
    Node,
    Direct,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ConsumerDeclaration {
    #[serde(default)]
    pub(crate) compatibility: Option<String>,
    pub(crate) metadata_path: String,
}

pub(crate) struct CatalogContext<'a> {
    pub(crate) marketplace_id: &'a LanguageMarketplaceId,
    pub(crate) package: PackageTargetMetadata,
    pub(crate) catalog: PackageCatalogMetadata,
    pub(crate) target_name: &'a str,
    pub(crate) target_length: u64,
    pub(crate) consumer_id: &'a str,
    pub(crate) consumer_version: &'a Version,
}

pub(crate) fn catalog_entries(
    context: CatalogContext<'_>,
) -> Result<Vec<LanguageMarketplaceEntry>, LanguageMarketplaceError> {
    let CatalogContext {
        marketplace_id,
        package,
        catalog,
        target_name,
        target_length,
        consumer_id,
        consumer_version,
    } = context;
    let manifest = catalog.manifest;
    if package.schema_version != 1
        || catalog.schema_version != 1
        || !matches!(manifest.schema_version, 1 | 2)
        || manifest.package_type != "language"
        || manifest.id != package.id
        || manifest.version != package.version
        || manifest.display_name.trim().is_empty()
        || manifest.description.trim().is_empty()
        || manifest.license.trim().is_empty()
        || manifest.source.trim().is_empty()
        || manifest.languages.is_empty()
        || catalog.package_file_count == 0
        || catalog.package_file_count > 10_000
        || catalog.package_size_bytes == 0
        || catalog.package_size_bytes > 256 * 1024 * 1024
        || target_length == 0
        || target_length > crate::archive::MAX_ARCHIVE_BYTES
    {
        return Err(metadata_error());
    }
    let expected_target = format!("packages/{}/{}.zip", package.id, package.version);
    if target_name != expected_target {
        return Err(metadata_error());
    }
    let compatibility = compatibility(&manifest, consumer_id, consumer_version)?;
    let executable_capabilities = manifest
        .capabilities
        .iter()
        .filter(|capability| capability.kind == "executable")
        .map(|capability| {
            validate_local_id(&capability.id)?;
            let path = validate_package_path(&capability.path)?;
            if !capability.path.starts_with("server/") {
                return Err(metadata_error());
            }
            let runtime = match (manifest.schema_version, capability.runtime) {
                (1, None) => LanguageMarketplaceRuntime::LegacyUnspecified,
                (2, Some(ExecutableRuntimeDocument::Node)) => LanguageMarketplaceRuntime::Node,
                (2, Some(ExecutableRuntimeDocument::Direct)) => LanguageMarketplaceRuntime::Direct,
                _ => return Err(metadata_error()),
            };
            Ok((capability.id.clone(), (path, runtime)))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let mut routes = BTreeMap::<String, (Vec<String>, Vec<String>)>::new();
    for language in &manifest.languages {
        validate_local_id(&language.id)?;
        if language.display_name.trim().is_empty()
            || language.aliases.iter().any(|alias| alias.trim().is_empty())
            || language
                .file_extensions
                .iter()
                .any(|extension| !valid_file_extension(extension))
        {
            return Err(metadata_error());
        }
        let server_id = match manifest.schema_version {
            1 if language.lsp && language.language_server.is_none() => {
                if executable_capabilities.len() != 1 {
                    return Err(metadata_error());
                }
                executable_capabilities
                    .keys()
                    .next()
                    .ok_or_else(metadata_error)?
                    .clone()
            }
            1 if !language.lsp && language.language_server.is_none() => continue,
            2 if !language.lsp => match &language.language_server {
                Some(server_id) => server_id.clone(),
                None => continue,
            },
            _ => return Err(metadata_error()),
        };
        if !executable_capabilities.contains_key(&server_id) {
            return Err(metadata_error());
        }
        let route = routes.entry(server_id).or_default();
        route.0.push(language.id.clone());
        route.1.extend(language.file_extensions.iter().cloned());
    }
    let mut entries = Vec::new();
    for (server_id, (mut languages, mut file_extensions)) in routes {
        languages.sort();
        languages.dedup();
        file_extensions.sort();
        file_extensions.dedup();
        let (executable_path, runtime) = executable_capabilities
            .get(&server_id)
            .cloned()
            .ok_or_else(metadata_error)?;
        entries.push(LanguageMarketplaceEntry {
            marketplace_id: marketplace_id.clone(),
            package_id: package.id.clone(),
            version: package.version.clone(),
            digest: package.package_digest.clone(),
            display_name: manifest.display_name.clone(),
            description: manifest.description.clone(),
            license: manifest.license.clone(),
            server_id,
            executable_path,
            runtime,
            languages,
            file_extensions,
            compatibility: compatibility.clone(),
            target_name: target_name.to_owned(),
            target_length,
            package_file_count: catalog.package_file_count,
            package_size_bytes: catalog.package_size_bytes,
        });
    }
    if entries.is_empty() {
        return Err(metadata_error());
    }
    Ok(entries)
}

fn compatibility(
    manifest: &PackageManifest,
    consumer_id: &str,
    consumer_version: &Version,
) -> Result<LanguageMarketplaceCompatibility, LanguageMarketplaceError> {
    let Some(consumer) = manifest.consumers.get(consumer_id) else {
        return Ok(LanguageMarketplaceCompatibility::Compatible);
    };
    let expected_metadata_path = format!(".marketplace/consumers/{consumer_id}.json");
    if consumer.metadata_path != expected_metadata_path {
        return Err(metadata_error());
    }
    let Some(requirement) = &consumer.compatibility else {
        return Ok(LanguageMarketplaceCompatibility::Compatible);
    };
    let requirement = VersionReq::parse(requirement).map_err(|_| metadata_error())?;
    if requirement.matches(consumer_version) {
        Ok(LanguageMarketplaceCompatibility::Compatible)
    } else {
        Ok(LanguageMarketplaceCompatibility::Incompatible(format!(
            "requires {consumer_id} {requirement}; this build is {consumer_version}"
        )))
    }
}

fn validate_local_id(value: &str) -> Result<(), LanguageMarketplaceError> {
    if valid_identifier_segment(value) {
        Ok(())
    } else {
        Err(metadata_error())
    }
}

fn valid_identifier_segment(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_LOCAL_ID_BYTES
        && value != "."
        && value != ".."
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn validate_package_path(value: &str) -> Result<PathBuf, LanguageMarketplaceError> {
    let path = Path::new(value);
    if value.is_empty()
        || value.len() > MAX_PATH_BYTES
        || value.contains('\\')
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(metadata_error());
    }
    Ok(path.to_path_buf())
}

fn valid_file_extension(value: &str) -> bool {
    value.starts_with('.')
        && value.len() > 1
        && value.len() <= 32
        && value
            .bytes()
            .skip(1)
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

pub(crate) fn metadata_error() -> LanguageMarketplaceError {
    LanguageMarketplaceError::new(
        LanguageMarketplaceErrorKind::MetadataUntrusted,
        "Language Marketplace signed metadata is invalid",
    )
}
