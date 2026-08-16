use serde_json::Value;
use zeta_app_server_protocol::protocol::common::EmptyParams;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceAcquireCapabilityParams;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceDownloadParams;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceGetParams;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceInstallParams;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceListInstalledResult;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceOpenResourceParams;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceReleaseCapabilityParams;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceSearchParams;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceUninstallModeDto;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceUninstallParams;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceUpdateParams;
use zeta_extensions::ExtensionCatalogReload;
use zeta_marketplace_client::AcquireCapabilityRequest;
use zeta_marketplace_client::CapabilityRef;
use zeta_marketplace_client::DownloadPackageRequest;
use zeta_marketplace_client::GetPackageRequest;
use zeta_marketplace_client::InstallPackageRequest;
use zeta_marketplace_client::ListInstalledRequest;
use zeta_marketplace_client::MarketplaceClientError;
use zeta_marketplace_client::MarketplaceClientErrorKind;
use zeta_marketplace_client::MarketplaceErrorCode;
use zeta_marketplace_client::OpenResourceRequest;
use zeta_marketplace_client::ReleaseCapabilityRequest;
use zeta_marketplace_client::ResourceRef;
use zeta_marketplace_client::SearchPackagesRequest;
use zeta_marketplace_client::UninstallMode;
use zeta_marketplace_client::UninstallPackageRequest;
use zeta_marketplace_client::UpdatePackageRequest;
use zeta_skills_extension::SkillCatalogReload;

use super::AppServer;
use super::ConnectionState;
use super::RpcError;
use super::decode;
use super::marketplace_projection;
use super::result;

impl AppServer {
    pub(super) fn marketplace_search(&self, params: &Value) -> Result<Value, RpcError> {
        let params: MarketplaceSearchParams = decode(params)?;
        let found = self
            .marketplace_manager()?
            .search(SearchPackagesRequest {
                query: params.query,
                package_type: params.package_type,
                limit: params.limit.map(|limit| limit as usize),
            })
            .map_err(marketplace_error)?;
        result(&marketplace_projection::search_result(found))
    }

    pub(super) fn marketplace_get(&self, params: &Value) -> Result<Value, RpcError> {
        let params: MarketplaceGetParams = decode(params)?;
        let details = self
            .marketplace_manager()?
            .get(GetPackageRequest {
                package_id: params.package_id,
                version: params.version,
            })
            .map_err(marketplace_error)?;
        result(&marketplace_projection::package_details(details))
    }

    pub(super) fn marketplace_download(&self, params: &Value) -> Result<Value, RpcError> {
        let params: MarketplaceDownloadParams = decode(params)?;
        let artifact = self
            .marketplace_manager()?
            .download(DownloadPackageRequest {
                package_id: params.package_id,
                version: params.version,
            })
            .map_err(marketplace_error)?;
        result(&marketplace_projection::artifact_handle(artifact))
    }

    pub(super) fn marketplace_install(&self, params: &Value) -> Result<Value, RpcError> {
        let params: MarketplaceInstallParams = decode(params)?;
        let installed = self
            .marketplace_manager()?
            .install(InstallPackageRequest {
                package_id: params.package_id,
                version: params.version,
            })
            .map_err(marketplace_error)?;
        self.reconcile_marketplace_consumers();
        result(&marketplace_projection::installed_package(installed))
    }

    pub(super) fn marketplace_update(&self, params: &Value) -> Result<Value, RpcError> {
        let params: MarketplaceUpdateParams = decode(params)?;
        let installed = self
            .marketplace_manager()?
            .update(UpdatePackageRequest {
                installation_id: params.installation_id,
                version: params.version,
            })
            .map_err(marketplace_error)?;
        self.reconcile_marketplace_consumers();
        result(&marketplace_projection::installed_package(installed))
    }

    pub(super) fn marketplace_uninstall(&self, params: &Value) -> Result<Value, RpcError> {
        let params: MarketplaceUninstallParams = decode(params)?;
        self.marketplace_manager()?
            .uninstall(UninstallPackageRequest {
                installation_id: params.installation_id,
                mode: match params.mode {
                    MarketplaceUninstallModeDto::IfUnused => UninstallMode::IfUnused,
                    MarketplaceUninstallModeDto::WhenUnused => UninstallMode::WhenUnused,
                },
            })
            .map_err(marketplace_error)?;
        self.reconcile_marketplace_consumers();
        result(&())
    }

    pub(super) fn marketplace_list_installed(&self, params: &Value) -> Result<Value, RpcError> {
        let _: EmptyParams = decode(params)?;
        let packages = self
            .marketplace_manager()?
            .list_installed(ListInstalledRequest {})
            .map_err(marketplace_error)?
            .into_iter()
            .map(marketplace_projection::installed_package)
            .collect();
        result(&MarketplaceListInstalledResult { packages })
    }

    pub(super) fn marketplace_acquire_capability(
        &self,
        connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: MarketplaceAcquireCapabilityParams = decode(params)?;
        let acquired = self
            .marketplace_manager()?
            .acquire_capability(AcquireCapabilityRequest {
                capability: CapabilityRef {
                    id: params.capability.id,
                },
            })
            .map_err(marketplace_error)?;
        connection.add_marketplace_lease(acquired.lease.id.clone());
        result(&marketplace_projection::acquired_capability(acquired))
    }

    pub(super) fn marketplace_release_capability(
        &self,
        connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: MarketplaceReleaseCapabilityParams = decode(params)?;
        require_owned_lease(connection, &params.lease_id)?;
        self.marketplace_manager()?
            .release_capability(ReleaseCapabilityRequest {
                lease_id: params.lease_id.clone(),
            })
            .map_err(marketplace_error)?;
        connection.remove_marketplace_lease(&params.lease_id);
        result(&())
    }

    pub(super) fn marketplace_open_resource(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: MarketplaceOpenResourceParams = decode(params)?;
        require_owned_lease(connection, &params.lease_id)?;
        let content = self
            .marketplace_manager()?
            .open_resource(OpenResourceRequest {
                lease_id: params.lease_id,
                resource: ResourceRef {
                    id: params.resource.id,
                },
            })
            .map_err(marketplace_error)?;
        result(&marketplace_projection::resource_content(content))
    }

    fn marketplace_manager(
        &self,
    ) -> Result<&dyn zeta_marketplace_client::MarketplaceServiceClient, RpcError> {
        self.marketplace_manager_client
            .as_deref()
            .ok_or_else(|| RpcError::new(-32100, AppServerErrorName::MarketplaceUnavailable))
    }

    fn reconcile_marketplace_consumers(&self) {
        if let Some(skills) = &self.skills
            && let Err(error) = skills.list(SkillCatalogReload::Refresh)
        {
            log::error!("failed to reconcile Marketplace Skills: {error}");
        }
        if let Ok(mut extensions) = self.extensions.lock() {
            extensions.list(ExtensionCatalogReload::Refresh);
        }
        if let Some(runtime) = &self.marketplace_language_runtime {
            match runtime.registry() {
                Ok(providers) => {
                    if let Ok(mut language) = self.language.lock() {
                        language.set_provider_registry(providers);
                    }
                }
                Err(error) => {
                    log::error!("failed to reconcile Marketplace language servers: {error}");
                }
            }
        }
    }
}

fn require_owned_lease(connection: &ConnectionState, lease_id: &str) -> Result<(), RpcError> {
    if connection.owns_marketplace_lease(lease_id) {
        Ok(())
    } else {
        Err(RpcError::new(
            -32101,
            AppServerErrorName::MarketplaceNotFound,
        ))
    }
}

fn marketplace_error(error: MarketplaceClientError) -> RpcError {
    let name = match error.kind() {
        MarketplaceClientErrorKind::Unavailable => AppServerErrorName::MarketplaceUnavailable,
        MarketplaceClientErrorKind::Protocol => AppServerErrorName::MarketplaceOperationFailed,
        MarketplaceClientErrorKind::Remote(code) => match code {
            MarketplaceErrorCode::PackageNotFound
            | MarketplaceErrorCode::VersionNotFound
            | MarketplaceErrorCode::CapabilityNotFound
            | MarketplaceErrorCode::LeaseNotFound
            | MarketplaceErrorCode::ResourceNotFound
            | MarketplaceErrorCode::InstallationNotFound => AppServerErrorName::MarketplaceNotFound,
            MarketplaceErrorCode::PackageUntrusted => AppServerErrorName::MarketplaceUntrusted,
            MarketplaceErrorCode::PackageIncompatible
            | MarketplaceErrorCode::CapabilityUnsupported => {
                AppServerErrorName::MarketplaceIncompatible
            }
            MarketplaceErrorCode::InstallationInUse => {
                AppServerErrorName::MarketplaceInstallationInUse
            }
            MarketplaceErrorCode::StorageUnavailable | MarketplaceErrorCode::ServiceUnavailable => {
                AppServerErrorName::MarketplaceUnavailable
            }
            MarketplaceErrorCode::InvalidRequest | MarketplaceErrorCode::MethodNotFound => {
                AppServerErrorName::MarketplaceOperationFailed
            }
        },
    };
    RpcError::new(-32101, name)
}
