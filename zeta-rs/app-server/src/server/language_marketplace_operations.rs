use serde_json::Value;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_app_server_protocol::protocol::language::LanguageMarketplaceInstallParams;
use zeta_app_server_protocol::protocol::language::LanguageMarketplaceInstallResult;

use super::AppServer;
use super::RpcError;
use super::decode;
use super::language_marketplace_runtime::LanguageMarketplaceRuntimeError;
use super::result;

impl AppServer {
    pub(super) fn language_marketplace_list(&self) -> Result<Value, RpcError> {
        let result_value = self
            .language_marketplaces()?
            .list()
            .map_err(runtime_error)?;
        result(&result_value)
    }

    pub(super) fn language_marketplace_install(&self, params: &Value) -> Result<Value, RpcError> {
        let params: LanguageMarketplaceInstallParams = decode(params)?;
        let (activation_generation, registry) = self
            .language_marketplaces()?
            .install(&params)
            .map_err(runtime_error)?;
        self.language
            .lock()
            .map_err(|_| operation_failed())?
            .set_provider_registry(registry);
        result(&LanguageMarketplaceInstallResult {
            activation_generation,
        })
    }

    fn language_marketplaces(
        &self,
    ) -> Result<&super::language_marketplace_runtime::AppServerLanguageMarketplaceRuntime, RpcError>
    {
        self.language_marketplaces.as_ref().ok_or_else(|| {
            RpcError::new(-32063, AppServerErrorName::LanguageMarketplaceUnavailable)
        })
    }
}

fn runtime_error(error: LanguageMarketplaceRuntimeError) -> RpcError {
    match error {
        LanguageMarketplaceRuntimeError::RevisionConflict => RpcError::new(
            -32064,
            AppServerErrorName::LanguageMarketplaceRevisionConflict,
        ),
        LanguageMarketplaceRuntimeError::Incompatible => {
            RpcError::new(-32065, AppServerErrorName::LanguageMarketplaceIncompatible)
        }
        LanguageMarketplaceRuntimeError::NotFound
        | LanguageMarketplaceRuntimeError::OperationFailed => operation_failed(),
    }
}

fn operation_failed() -> RpcError {
    RpcError::new(
        -32066,
        AppServerErrorName::LanguageMarketplaceOperationFailed,
    )
}
