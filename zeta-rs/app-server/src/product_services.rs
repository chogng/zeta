use std::collections::BTreeSet;
use std::fs;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;
use url::Url;
use zeta_connectors::ConnectorId;
use zeta_connectors_extension::GitHubBrokeredOAuthConfig;
use zeta_connectors_extension::GitHubDeviceOAuthConfig;

use crate::OpenAppServerError;

const PRODUCT_SERVICES_SCHEMA_VERSION: u32 = 1;
const MAX_PRODUCT_SERVICES_BYTES: u64 = 1024 * 1024;

/// Product-distribution trust and public OAuth configuration loaded by a host.
///
/// This document contains no confidential client secret. Marketplace root metadata remains pinned
/// by the product file, while broker URLs and public client IDs are explicit host inputs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalProductServicesConfig {
    pub(crate) marketplace_registry: Option<zeta_marketplace_client::RemoteMarketplaceConfig>,
    pub(crate) connector_oauth: Vec<ProductConnectorOAuthConfig>,
}

impl LocalProductServicesConfig {
    pub fn load(
        path: impl AsRef<Path>,
        profile_root: impl AsRef<Path>,
    ) -> Result<Self, OpenAppServerError> {
        let path = path.as_ref();
        let metadata = fs::symlink_metadata(path).map_err(product_config_error)?;
        if metadata.file_type().is_symlink()
            || !metadata.is_file()
            || metadata.len() > MAX_PRODUCT_SERVICES_BYTES
        {
            return Err(product_config_error(()));
        }
        let bytes = fs::read(path).map_err(product_config_error)?;
        let document: ProductServicesDocument =
            serde_json::from_slice(&bytes).map_err(product_config_error)?;
        if document.schema_version != PRODUCT_SERVICES_SCHEMA_VERSION {
            return Err(product_config_error(()));
        }
        let source_root = path.parent().ok_or_else(|| product_config_error(()))?;
        let marketplace_registry = document
            .marketplace_manager
            .map(|manager| {
                let trusted_root = read_trusted_root(source_root, &manager.trusted_root)?;
                let config = zeta_marketplace_client::RemoteMarketplaceConfig::new(
                    Url::parse(&manager.metadata_base_url).map_err(product_config_error)?,
                    Url::parse(&manager.targets_base_url).map_err(product_config_error)?,
                    trusted_root,
                    profile_root.as_ref().join("cache/marketplace"),
                )
                .map_err(product_config_error)?;
                if manager.allowed_publishers.is_empty() {
                    Ok(config)
                } else {
                    config
                        .with_allowed_publishers(manager.allowed_publishers)
                        .map_err(product_config_error)
                }
            })
            .transpose()?;
        let connector_oauth = document
            .connector_oauth
            .into_iter()
            .map(ProductConnectorOAuthConfig::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        validate_unique_configuration(&connector_oauth)?;
        Ok(Self {
            marketplace_registry,
            connector_oauth,
        })
    }

    /// Returns the product-pinned remote registry configuration used by Marketplace Manager.
    pub fn marketplace_registry(
        &self,
    ) -> Option<&zeta_marketplace_client::RemoteMarketplaceConfig> {
        self.marketplace_registry.as_ref()
    }
}

fn validate_unique_configuration(
    connector_oauth: &[ProductConnectorOAuthConfig],
) -> Result<(), OpenAppServerError> {
    let connector_ids = connector_oauth
        .iter()
        .map(|configuration| match configuration {
            ProductConnectorOAuthConfig::GitHubBrokered { connector_id, .. }
            | ProductConnectorOAuthConfig::GitHubDevice { connector_id, .. } => connector_id,
        })
        .collect::<BTreeSet<_>>();
    if connector_ids.len() != connector_oauth.len() {
        return Err(product_config_error(()));
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ProductConnectorOAuthConfig {
    GitHubBrokered {
        connector_id: ConnectorId,
        config: GitHubBrokeredOAuthConfig,
    },
    GitHubDevice {
        connector_id: ConnectorId,
        config: GitHubDeviceOAuthConfig,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProductServicesDocument {
    schema_version: u32,
    #[serde(default)]
    marketplace_manager: Option<ProductMarketplaceManagerDocument>,
    #[serde(default)]
    connector_oauth: Vec<ProductConnectorOAuthDocument>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProductMarketplaceManagerDocument {
    metadata_base_url: String,
    targets_base_url: String,
    trusted_root: PathBuf,
    #[serde(default)]
    allowed_publishers: Vec<String>,
}

#[derive(Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum ProductConnectorOAuthDocument {
    #[serde(rename = "githubBrokered")]
    GitHubBrokered {
        connector_id: String,
        broker_base_url: String,
        client_id: String,
        #[serde(default)]
        scopes: Vec<String>,
    },
    #[serde(rename = "githubDevice")]
    GitHubDevice {
        connector_id: String,
        client_id: String,
        #[serde(default)]
        scopes: Vec<String>,
    },
}

impl TryFrom<ProductConnectorOAuthDocument> for ProductConnectorOAuthConfig {
    type Error = OpenAppServerError;

    fn try_from(document: ProductConnectorOAuthDocument) -> Result<Self, Self::Error> {
        match document {
            ProductConnectorOAuthDocument::GitHubBrokered {
                connector_id,
                broker_base_url,
                client_id,
                scopes,
            } => Ok(Self::GitHubBrokered {
                connector_id: ConnectorId::new(connector_id).map_err(product_config_error)?,
                config: GitHubBrokeredOAuthConfig {
                    broker_base_url: Url::parse(&broker_base_url).map_err(product_config_error)?,
                    client_id,
                    scopes,
                },
            }),
            ProductConnectorOAuthDocument::GitHubDevice {
                connector_id,
                client_id,
                scopes,
            } => Ok(Self::GitHubDevice {
                connector_id: ConnectorId::new(connector_id).map_err(product_config_error)?,
                config: GitHubDeviceOAuthConfig { client_id, scopes },
            }),
        }
    }
}

fn read_trusted_root(root: &Path, relative_path: &Path) -> Result<Vec<u8>, OpenAppServerError> {
    let path = resolve_relative_regular_file(root, relative_path)?;
    fs::read(path).map_err(product_config_error)
}

fn resolve_relative_regular_file(
    root: &Path,
    relative_path: &Path,
) -> Result<PathBuf, OpenAppServerError> {
    if relative_path.as_os_str().is_empty()
        || relative_path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(product_config_error(()));
    }
    let canonical_root = fs::canonicalize(root).map_err(product_config_error)?;
    let path = root.join(relative_path);
    let metadata = fs::symlink_metadata(&path).map_err(product_config_error)?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > MAX_PRODUCT_SERVICES_BYTES
    {
        return Err(product_config_error(()));
    }
    let canonical_path = fs::canonicalize(&path).map_err(product_config_error)?;
    if !canonical_path.starts_with(&canonical_root) {
        return Err(product_config_error(()));
    }
    Ok(canonical_path)
}

fn product_config_error(_: impl Sized) -> OpenAppServerError {
    OpenAppServerError("product services configuration is invalid".into())
}

#[cfg(test)]
#[path = "product_services_tests.rs"]
mod tests;
