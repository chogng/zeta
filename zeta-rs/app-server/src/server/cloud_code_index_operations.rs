use std::num::NonZeroU64;

use serde_json::Value;
use zeta_app_server_protocol::protocol::code_index::CloudCodeIndexAuthorizeParams;
use zeta_app_server_protocol::protocol::code_index::CloudCodeIndexDestinationDto;
use zeta_app_server_protocol::protocol::code_index::CloudCodeIndexGrantDto;
use zeta_app_server_protocol::protocol::code_index::CloudCodeIndexPreviewParams;
use zeta_app_server_protocol::protocol::code_index::CloudCodeIndexPreviewResult;
use zeta_app_server_protocol::protocol::code_index::CloudCodeIndexSelectionDto;
use zeta_app_server_protocol::protocol::code_index::CloudCodeIndexStateDto;
use zeta_app_server_protocol::protocol::code_index::CloudCodeIndexStatusResult;
use zeta_app_server_protocol::protocol::code_index::CodeIndexDeploymentModeDto;
use zeta_app_server_protocol::protocol::common::EmptyParams;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_code_index_cloud::CloudCodeIndexDestination;
use zeta_code_index_cloud::CloudCodeIndexError;
use zeta_code_index_cloud::CloudCodeIndexGrant;
use zeta_code_index_cloud::CloudCodeIndexGrantId;
use zeta_code_index_cloud::CloudCodeIndexLimitDisposition;
use zeta_code_index_cloud::CloudCodeIndexPreview;
use zeta_code_index_cloud::CloudCodeIndexProviderId;
use zeta_code_index_cloud::CloudCodeIndexSelection;
use zeta_code_index_cloud::CloudCodeIndexState;
use zeta_code_index_cloud::CloudCodeIndexStatus;
use zeta_code_index_cloud::CodeIndexDeploymentMode;

use super::AppServer;
use super::RpcError;
use super::decode;
use super::result;

impl AppServer {
    pub(super) fn cloud_code_index_status(&self, params: &Value) -> Result<Value, RpcError> {
        let _: EmptyParams = decode(params)?;
        let controller = self.cloud_code_index_service()?;
        let status = controller.status().map_err(cloud_code_index_error)?;
        result(&project_status(&status, controller.root_id().as_str()))
    }

    pub(super) fn cloud_code_index_preview(&self, params: &Value) -> Result<Value, RpcError> {
        let params: CloudCodeIndexPreviewParams = decode(params)?;
        let selection = selection(params.selection)?;
        let max_egress_bytes = NonZeroU64::new(params.max_egress_bytes)
            .ok_or_else(|| RpcError::new(-32602, AppServerErrorName::InvalidParams))?;
        let controller = self.cloud_code_index_service()?;
        let preview = controller
            .preview(&selection, max_egress_bytes)
            .map_err(cloud_code_index_error)?;
        result(&project_preview(preview))
    }

    pub(super) fn cloud_code_index_authorize(&self, params: &Value) -> Result<Value, RpcError> {
        let params: CloudCodeIndexAuthorizeParams = decode(params)?;
        let controller = self.cloud_code_index_service()?;
        let max_egress_bytes = NonZeroU64::new(params.grant.max_egress_bytes)
            .ok_or_else(|| RpcError::new(-32602, AppServerErrorName::InvalidParams))?;
        let destination = CloudCodeIndexDestination::new(
            CloudCodeIndexProviderId::new(params.grant.destination.provider)
                .map_err(cloud_code_index_error)?,
            params.grant.destination.tenant,
            params.grant.destination.collection,
        )
        .map_err(cloud_code_index_error)?;
        let grant = CloudCodeIndexGrant {
            id: CloudCodeIndexGrantId::new(params.grant.grant_id)
                .map_err(cloud_code_index_error)?,
            root_id: controller.root_id().as_str().to_owned(),
            destination,
            selection: selection(params.grant.selection)?,
            max_egress_bytes,
        };
        let status = controller
            .authorize(grant)
            .map_err(cloud_code_index_error)?;
        result(&project_status(&status, controller.root_id().as_str()))
    }

    pub(super) fn cloud_code_index_sync(&self, params: &Value) -> Result<Value, RpcError> {
        let _: EmptyParams = decode(params)?;
        let controller = self.cloud_code_index_service()?;
        let status = controller.sync().map_err(cloud_code_index_error)?;
        result(&project_status(&status, controller.root_id().as_str()))
    }

    pub(super) fn cloud_code_index_revoke(&self, params: &Value) -> Result<Value, RpcError> {
        let _: EmptyParams = decode(params)?;
        let controller = self.cloud_code_index_service()?;
        let status = controller.revoke().map_err(cloud_code_index_error)?;
        result(&project_status(&status, controller.root_id().as_str()))
    }
}

fn selection(value: CloudCodeIndexSelectionDto) -> Result<CloudCodeIndexSelection, RpcError> {
    match value {
        CloudCodeIndexSelectionDto::EntireIndex => Ok(CloudCodeIndexSelection::EntireIndex),
        CloudCodeIndexSelectionDto::PathPrefixes { prefixes } => {
            CloudCodeIndexSelection::path_prefixes(prefixes).map_err(cloud_code_index_error)
        }
    }
}

fn project_preview(preview: CloudCodeIndexPreview) -> CloudCodeIndexPreviewResult {
    CloudCodeIndexPreviewResult {
        local_generation: preview.local_generation,
        file_count: preview.file_count,
        chunk_count: preview.chunk_count,
        upload_unit_count: preview.upload_unit_count,
        egress_bytes: preview.egress_bytes,
        within_limit: preview.limit == CloudCodeIndexLimitDisposition::WithinLimit,
    }
}

fn project_status(status: &CloudCodeIndexStatus, root_id: &str) -> CloudCodeIndexStatusResult {
    CloudCodeIndexStatusResult {
        deployment_mode: match status.deployment_mode {
            CodeIndexDeploymentMode::LocalOnly => CodeIndexDeploymentModeDto::LocalOnly,
            CodeIndexDeploymentMode::Cloud => CodeIndexDeploymentModeDto::Cloud,
        },
        state: match status.state {
            CloudCodeIndexState::LocalOnly => CloudCodeIndexStateDto::LocalOnly,
            CloudCodeIndexState::Granted => CloudCodeIndexStateDto::Granted,
            CloudCodeIndexState::Syncing => CloudCodeIndexStateDto::Syncing,
            CloudCodeIndexState::Ready => CloudCodeIndexStateDto::Ready,
            CloudCodeIndexState::Stale => CloudCodeIndexStateDto::Stale,
            CloudCodeIndexState::Revoking => CloudCodeIndexStateDto::Revoking,
            CloudCodeIndexState::Failed => CloudCodeIndexStateDto::Failed,
        },
        root_id: root_id.to_owned(),
        grant: status.grant.as_ref().map(project_grant),
        local_generation: status.local_generation,
        synced_local_generation: status.synced_local_generation,
        remote_generation: status.remote_generation.clone(),
    }
}

fn project_grant(grant: &CloudCodeIndexGrant) -> CloudCodeIndexGrantDto {
    CloudCodeIndexGrantDto {
        grant_id: grant.id.as_str().to_owned(),
        destination: CloudCodeIndexDestinationDto {
            provider: grant.destination.provider.as_str().to_owned(),
            tenant: grant.destination.tenant.clone(),
            collection: grant.destination.collection.clone(),
        },
        selection: match &grant.selection {
            CloudCodeIndexSelection::EntireIndex => CloudCodeIndexSelectionDto::EntireIndex,
            CloudCodeIndexSelection::PathPrefixes(prefixes) => {
                CloudCodeIndexSelectionDto::PathPrefixes {
                    prefixes: prefixes.clone(),
                }
            }
        },
        max_egress_bytes: grant.max_egress_bytes.get(),
    }
}

fn cloud_code_index_error(error: CloudCodeIndexError) -> RpcError {
    match error {
        CloudCodeIndexError::InvalidInput(_) | CloudCodeIndexError::StorageRootMismatch => {
            RpcError::new(-32602, AppServerErrorName::CloudCodeIndexInvalidGrant)
        }
        CloudCodeIndexError::ConsentConflict => {
            RpcError::new(-32094, AppServerErrorName::CloudCodeIndexConsentConflict)
        }
        CloudCodeIndexError::EgressLimitExceeded => RpcError::new(
            -32095,
            AppServerErrorName::CloudCodeIndexEgressLimitExceeded,
        ),
        CloudCodeIndexError::LocalIndexNotReady => {
            RpcError::new(-32091, AppServerErrorName::CodeIndexNotReady)
        }
        CloudCodeIndexError::ProviderUnavailable | CloudCodeIndexError::DeletionUnsupported => {
            RpcError::new(
                -32096,
                AppServerErrorName::CloudCodeIndexProviderUnavailable,
            )
        }
        CloudCodeIndexError::NoActiveGrant
        | CloudCodeIndexError::InvalidState
        | CloudCodeIndexError::InvalidProviderResult(_)
        | CloudCodeIndexError::Provider(_)
        | CloudCodeIndexError::LocalIndex(_)
        | CloudCodeIndexError::IncompatibleStorage
        | CloudCodeIndexError::Storage(_)
        | CloudCodeIndexError::Serialization(_)
        | CloudCodeIndexError::Io { .. } => {
            RpcError::new(-32097, AppServerErrorName::CloudCodeIndexOperationFailed)
        }
    }
}

#[cfg(test)]
#[path = "cloud_code_index_operations_tests.rs"]
mod tests;
