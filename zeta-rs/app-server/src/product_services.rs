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
use zeta_language_marketplace::LanguageMarketplaceId;
use zeta_language_marketplace::RemoteLanguageMarketplaceConfig;
use zeta_plugin_marketplace::RemotePluginMarketplaceConfig;
use zeta_plugins::PluginMarketplaceId;

use crate::OpenAppServerError;

const PRODUCT_SERVICES_SCHEMA_VERSION: u32 = 1;
const MAX_PRODUCT_SERVICES_BYTES: u64 = 1024 * 1024;

/// Product-distribution trust and public OAuth configuration loaded by a host.
///
/// This document contains no confidential client secret. Marketplace root metadata remains pinned
/// by the product file, while broker URLs and public client IDs are explicit host inputs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalProductServicesConfig {
    pub(crate) marketplaces: Vec<RemotePluginMarketplaceConfig>,
    pub(crate) language_marketplaces: Vec<RemoteLanguageMarketplaceConfig>,
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
        let marketplace_configs = document
            .marketplaces
            .into_iter()
            .map(|marketplace| {
                let plugin_id = PluginMarketplaceId::new(marketplace.id.clone())
                    .map_err(|_| product_config_error(()))?;
                let language_id = LanguageMarketplaceId::new(marketplace.id)
                    .map_err(|_| product_config_error(()))?;
                let trusted_root = read_trusted_root(source_root, &marketplace.trusted_root)?;
                let plugin_cache_root = profile_root
                    .as_ref()
                    .join("cache/plugin-marketplaces")
                    .join(plugin_id.as_str());
                let language_cache_root = profile_root
                    .as_ref()
                    .join("cache/language-marketplaces")
                    .join(language_id.as_str());
                let plugin = RemotePluginMarketplaceConfig::new(
                    plugin_id,
                    Url::parse(&marketplace.metadata_base_url).map_err(product_config_error)?,
                    Url::parse(&marketplace.targets_base_url).map_err(product_config_error)?,
                    trusted_root.clone(),
                    plugin_cache_root,
                )
                .map_err(product_config_error)?;
                let language = RemoteLanguageMarketplaceConfig::new(
                    language_id,
                    Url::parse(&marketplace.metadata_base_url).map_err(product_config_error)?,
                    Url::parse(&marketplace.targets_base_url).map_err(product_config_error)?,
                    trusted_root,
                    language_cache_root,
                    "zeta",
                    semver::Version::parse(env!("CARGO_PKG_VERSION"))
                        .expect("App Server package version is SemVer"),
                )
                .map_err(product_config_error)?;
                match marketplace.trust {
                    ProductMarketplaceTrustDocument::ProductManaged
                        if marketplace.allowed_publishers.is_empty() =>
                    {
                        Ok((plugin, language))
                    }
                    ProductMarketplaceTrustDocument::VerifiedExternal => {
                        let publishers = marketplace.allowed_publishers;
                        Ok((
                            plugin
                                .with_verified_external_publishers(publishers.clone())
                                .map_err(product_config_error)?,
                            language
                                .with_allowed_publishers(publishers)
                                .map_err(product_config_error)?,
                        ))
                    }
                    ProductMarketplaceTrustDocument::ProductManaged => {
                        Err(product_config_error(()))
                    }
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        let (marketplaces, language_marketplaces): (
            Vec<RemotePluginMarketplaceConfig>,
            Vec<RemoteLanguageMarketplaceConfig>,
        ) = marketplace_configs.into_iter().unzip();
        let connector_oauth = document
            .connector_oauth
            .into_iter()
            .map(ProductConnectorOAuthConfig::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        validate_unique_configuration(&marketplaces, &connector_oauth)?;
        Ok(Self {
            marketplaces,
            language_marketplaces,
            connector_oauth,
        })
    }
}

fn validate_unique_configuration(
    marketplaces: &[RemotePluginMarketplaceConfig],
    connector_oauth: &[ProductConnectorOAuthConfig],
) -> Result<(), OpenAppServerError> {
    let marketplace_ids = marketplaces
        .iter()
        .map(|marketplace| marketplace.id())
        .collect::<BTreeSet<_>>();
    let connector_ids = connector_oauth
        .iter()
        .map(|configuration| match configuration {
            ProductConnectorOAuthConfig::GitHubBrokered { connector_id, .. }
            | ProductConnectorOAuthConfig::GitHubDevice { connector_id, .. } => connector_id,
        })
        .collect::<BTreeSet<_>>();
    if marketplace_ids.len() != marketplaces.len() || connector_ids.len() != connector_oauth.len() {
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
    marketplaces: Vec<ProductMarketplaceDocument>,
    #[serde(default)]
    connector_oauth: Vec<ProductConnectorOAuthDocument>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProductMarketplaceDocument {
    id: String,
    #[serde(default)]
    trust: ProductMarketplaceTrustDocument,
    #[serde(default)]
    allowed_publishers: Vec<String>,
    metadata_base_url: String,
    targets_base_url: String,
    trusted_root: PathBuf,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
enum ProductMarketplaceTrustDocument {
    #[default]
    ProductManaged,
    VerifiedExternal,
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
    fs::read(canonical_path).map_err(product_config_error)
}

fn product_config_error(_: impl Sized) -> OpenAppServerError {
    OpenAppServerError("product services configuration is invalid".into())
}

#[cfg(test)]
#[path = "product_services_tests.rs"]
mod tests;
