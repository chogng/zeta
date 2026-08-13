use super::AppServer;
use super::RpcError;
use super::decode;
use super::result;
use serde_json::Value;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_app_server_protocol::protocol::plugins::PluginCommandDispositionDto;
use zeta_app_server_protocol::protocol::plugins::PluginCommandResultDto;
use zeta_app_server_protocol::protocol::plugins::PluginListResult;
use zeta_app_server_protocol::protocol::plugins::PluginMarketplaceCommandParams;
use zeta_app_server_protocol::protocol::plugins::PluginMarketplaceListResult;
use zeta_app_server_protocol::protocol::plugins::PluginMarketplaceModeDto;
use zeta_app_server_protocol::protocol::plugins::PluginMarketplacePackageDto;
use zeta_app_server_protocol::protocol::plugins::PluginPackageCommandParams;
use zeta_app_server_protocol::protocol::plugins::PluginPackageDto;
use zeta_plugins::InstalledPluginRef;
use zeta_plugins::PluginAuthorityCommand;
use zeta_plugins::PluginAuthorityCommandId;
use zeta_plugins::PluginAuthorityCommandRequest;
use zeta_plugins::PluginAuthorityCommandResult;
use zeta_plugins::PluginAuthorityDisposition;
use zeta_plugins::PluginError;
use zeta_plugins::PluginErrorKind;
use zeta_plugins::PluginId;
use zeta_plugins::PluginMarketplaceId;
use zeta_plugins::PluginMarketplaceMode;
use zeta_plugins::PluginPackageDigest;
use zeta_plugins::PluginVersion;

impl AppServer {
    pub(super) fn plugin_list(&self) -> Result<Value, RpcError> {
        let snapshot = self.plugin_authority()?.snapshot();
        let packages = snapshot
            .installed()
            .iter()
            .map(|package| PluginPackageDto {
                id: package.id.as_str().to_owned(),
                version: package.version.to_string(),
                digest: package.digest.as_str().to_owned(),
                enabled: snapshot.enabled().contains(package),
                granted: snapshot.granted().contains(package),
                revoked: snapshot.revoked().contains(package),
                effective: snapshot.activation().packages().iter().any(|active| {
                    active.manifest().id == package.id
                        && active.manifest().version == package.version
                        && active.package_digest() == &package.digest
                }),
            })
            .collect();
        result(&PluginListResult {
            revision: snapshot.revision(),
            activation_generation: snapshot.activation().generation(),
            packages,
        })
    }

    pub(super) fn plugin_enable(&self, params: &Value) -> Result<Value, RpcError> {
        self.plugin_package_command(params, |package| PluginAuthorityCommand::Enable { package })
    }

    pub(super) fn plugin_marketplace_list(&self) -> Result<Value, RpcError> {
        let Some(service) = &self.plugin_marketplaces else {
            return result(&PluginMarketplaceListResult {
                packages: Vec::new(),
            });
        };
        let installed = service.authority().snapshot();
        let packages = service
            .marketplaces()
            .flat_map(|marketplace| {
                marketplace.list().into_iter().map(|package| {
                    let package = package.package_ref();
                    PluginMarketplacePackageDto {
                        marketplace_id: marketplace.id().as_str().to_owned(),
                        marketplace_revision: marketplace.revision().as_str().to_owned(),
                        marketplace_mode: match marketplace.mode() {
                            PluginMarketplaceMode::Managed => PluginMarketplaceModeDto::Managed,
                            PluginMarketplaceMode::RemoteManaged => {
                                PluginMarketplaceModeDto::RemoteManaged
                            }
                            PluginMarketplaceMode::LocalDevelopment => {
                                PluginMarketplaceModeDto::LocalDevelopment
                            }
                        },
                        id: package.id.as_str().to_owned(),
                        version: package.version.to_string(),
                        digest: package.digest.as_str().to_owned(),
                        installed: installed.installed().contains(&package),
                    }
                })
            })
            .collect();
        result(&PluginMarketplaceListResult { packages })
    }

    pub(super) fn plugin_install(&self, params: &Value) -> Result<Value, RpcError> {
        self.plugin_marketplace_command(params, false)
    }

    pub(super) fn plugin_update(&self, params: &Value) -> Result<Value, RpcError> {
        self.plugin_marketplace_command(params, true)
    }

    pub(super) fn plugin_rollback(&self, params: &Value) -> Result<Value, RpcError> {
        let params: PluginPackageCommandParams = decode(params)?;
        let outcome = self
            .plugin_marketplaces()?
            .rollback(
                PluginAuthorityCommandId::new(params.command_id).map_err(plugin_error)?,
                params.expected_revision,
                package_ref(params.id, params.version, params.digest)?,
            )
            .map_err(plugin_error)?;
        result(&plugin_command_result(outcome))
    }

    fn plugin_marketplace_command(&self, params: &Value, update: bool) -> Result<Value, RpcError> {
        let params: PluginMarketplaceCommandParams = decode(params)?;
        let command_id = PluginAuthorityCommandId::new(params.command_id).map_err(plugin_error)?;
        let marketplace_id =
            PluginMarketplaceId::new(params.marketplace_id).map_err(plugin_error)?;
        let package = package_ref(params.id, params.version, params.digest)?;
        let outcome = if update {
            self.plugin_marketplaces()?.stage_update(
                command_id,
                params.expected_revision,
                &marketplace_id,
                &package,
            )
        } else {
            self.plugin_marketplaces()?.install(
                command_id,
                params.expected_revision,
                &marketplace_id,
                &package,
            )
        }
        .map_err(plugin_error)?;
        result(&plugin_command_result(outcome.command))
    }

    pub(super) fn plugin_disable(&self, params: &Value) -> Result<Value, RpcError> {
        self.plugin_package_command(params, |package| PluginAuthorityCommand::Disable {
            package,
        })
    }

    pub(super) fn plugin_grant(&self, params: &Value) -> Result<Value, RpcError> {
        self.plugin_package_command(params, |package| PluginAuthorityCommand::Grant { package })
    }

    pub(super) fn plugin_revoke_grant(&self, params: &Value) -> Result<Value, RpcError> {
        self.plugin_package_command(params, |package| PluginAuthorityCommand::RevokeGrant {
            package,
        })
    }

    pub(super) fn plugin_uninstall(&self, params: &Value) -> Result<Value, RpcError> {
        self.plugin_package_command(params, |package| PluginAuthorityCommand::Uninstall {
            package,
        })
    }

    fn plugin_package_command(
        &self,
        params: &Value,
        command: impl FnOnce(InstalledPluginRef) -> PluginAuthorityCommand,
    ) -> Result<Value, RpcError> {
        let params: PluginPackageCommandParams = decode(params)?;
        let package = package_ref(params.id, params.version, params.digest)?;
        let outcome = self
            .plugin_authority()?
            .apply(PluginAuthorityCommandRequest {
                command_id: PluginAuthorityCommandId::new(params.command_id)
                    .map_err(plugin_error)?,
                expected_revision: params.expected_revision,
                command: command(package),
            })
            .map_err(plugin_error)?;
        result(&plugin_command_result(outcome))
    }

    fn plugin_authority(&self) -> Result<&zeta_plugins::PluginActivationAuthority, RpcError> {
        self.plugins
            .as_ref()
            .ok_or_else(|| RpcError::new(-32040, AppServerErrorName::PluginsUnavailable))
    }

    fn plugin_marketplaces(&self) -> Result<&zeta_plugins::PluginMarketplaceService, RpcError> {
        self.plugin_marketplaces
            .as_ref()
            .ok_or_else(|| RpcError::new(-32040, AppServerErrorName::PluginsUnavailable))
    }
}

fn package_ref(
    id: String,
    version: String,
    digest: String,
) -> Result<InstalledPluginRef, RpcError> {
    Ok(InstalledPluginRef {
        id: PluginId::new(id).map_err(|_| invalid_params())?,
        version: PluginVersion::new(version).map_err(|_| invalid_params())?,
        digest: PluginPackageDigest::new(digest).map_err(|_| invalid_params())?,
    })
}

fn plugin_command_result(result: PluginAuthorityCommandResult) -> PluginCommandResultDto {
    PluginCommandResultDto {
        revision: result.revision,
        activation_generation: result.activation_generation,
        disposition: match result.disposition {
            PluginAuthorityDisposition::Updated => PluginCommandDispositionDto::Updated,
            PluginAuthorityDisposition::Replayed => PluginCommandDispositionDto::Replayed,
        },
    }
}

fn plugin_error(error: PluginError) -> RpcError {
    match error.kind() {
        PluginErrorKind::GenerationConflict => {
            RpcError::new(-32041, AppServerErrorName::PluginRevisionConflict)
        }
        PluginErrorKind::CommandConflict => {
            RpcError::new(-32004, AppServerErrorName::CommandConflict)
        }
        PluginErrorKind::SourceUnavailable
        | PluginErrorKind::PackageUnsafe
        | PluginErrorKind::ManifestInvalid
        | PluginErrorKind::ContributionInvalid
        | PluginErrorKind::PackageConflict
        | PluginErrorKind::AuthorityUnavailable
        | PluginErrorKind::PackageInUse
        | PluginErrorKind::PackageRevoked => {
            RpcError::new(-32042, AppServerErrorName::PluginOperationFailed)
        }
    }
}

fn invalid_params() -> RpcError {
    RpcError::new(-32602, AppServerErrorName::InvalidParams)
}

#[cfg(test)]
#[path = "plugin_operations_tests.rs"]
mod tests;
