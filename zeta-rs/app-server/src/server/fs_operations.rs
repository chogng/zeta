use super::operations::resource_rpc_error;
use super::{AppServer, ConnectionState, RpcError, decode, result};
use crate::resource_store::MAX_RESOURCE_BYTES;
use serde_json::Value;
use std::time::Duration;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_app_server_protocol::protocol::fs::{
    FsCreateFileParams, FsDeleteMode, FsDeleteParams, FsExistingTargetBehavior, FsFileType,
    FsGetMetadataParams, FsGetMetadataResult, FsMissingTargetBehavior, FsReadBinaryFileParams,
    FsReadBinaryFileResult, FsReadDirectoryEntry, FsReadDirectoryParams, FsReadDirectoryResult,
    FsReadFileParams, FsReadFileResult, FsRenameParams, FsWriteFileParams, FsWriteFileResult,
};
use zeta_app_server_protocol::protocol::resources::ResourceMetadataResult;
use zeta_file_system::ExistingTargetBehavior;
use zeta_file_system::FileDeleteMode;
use zeta_file_system::FileMetadata;
use zeta_file_system::FileSystemError;
use zeta_file_system::FileType;
use zeta_file_system::FileWriteCondition;
use zeta_file_system::MissingTargetBehavior;
use zeta_file_system::file_revision;

const MAX_EDITOR_FILE_BYTES: usize = 50 * 1024 * 1024;
const BINARY_PREVIEW_RESOURCE_TTL: Duration = Duration::from_secs(300);

impl AppServer {
    pub(super) fn fs_get_metadata(&self, params: &Value) -> Result<Value, RpcError> {
        let params: FsGetMetadataParams = decode(params)?;
        let metadata = self
            .file_system_service()?
            .get_metadata(&params.path)
            .map_err(file_system_error)?;
        result(&metadata_result(metadata))
    }

    pub(super) fn fs_read_directory(&self, params: &Value) -> Result<Value, RpcError> {
        let params: FsReadDirectoryParams = decode(params)?;
        let entries = self
            .file_system_service()?
            .read_directory(&params.path)
            .map_err(file_system_error)?
            .into_iter()
            .map(|entry| FsReadDirectoryEntry {
                name: entry.name,
                file_type: file_type(entry.file_type),
            })
            .collect();
        result(&FsReadDirectoryResult { entries })
    }

    pub(super) fn fs_read_file(&self, params: &Value) -> Result<Value, RpcError> {
        let params: FsReadFileParams = decode(params)?;
        let content = self
            .file_system_service()?
            .read_file_with_revision(&params.path, MAX_EDITOR_FILE_BYTES)
            .map_err(file_system_error)?;
        let text = String::from_utf8(content.bytes).map_err(|_| {
            file_system_error(FileSystemError::Io("file is not valid UTF-8".into()))
        })?;
        result(&FsReadFileResult {
            content: text,
            revision: content.revision,
        })
    }

    pub(super) fn fs_read_binary_file(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: FsReadBinaryFileParams = decode(params)?;
        let content = self
            .file_system_service()?
            .read_file_with_revision(&params.path, MAX_RESOURCE_BYTES)
            .map_err(file_system_error)?;
        let metadata = self
            .resources
            .lock()
            .map_err(|_| RpcError::new(-32000, AppServerErrorName::ServerOverloaded))?
            .create(
                connection.connection_id,
                "application/octet-stream".into(),
                content.bytes,
                BINARY_PREVIEW_RESOURCE_TTL,
            )
            .map_err(resource_rpc_error)?;
        result(&FsReadBinaryFileResult {
            resource: ResourceMetadataResult {
                resource_id: metadata.resource_id,
                mime_type: metadata.mime_type,
                size: metadata.size,
                sha256: metadata.sha256,
            },
            revision: content.revision,
        })
    }

    pub(super) fn fs_write_file(&self, params: &Value) -> Result<Value, RpcError> {
        let params: FsWriteFileParams = decode(params)?;
        let condition = match params.expected_revision {
            Some(revision) => FileWriteCondition::ExpectedRevision(revision),
            None => FileWriteCondition::Unconditional,
        };
        let metadata = self
            .file_system_service()?
            .write_file_with_condition(
                &params.path,
                params.content.as_bytes(),
                MAX_EDITOR_FILE_BYTES,
                &condition,
            )
            .map_err(file_system_error)?;
        result(&FsWriteFileResult {
            metadata: metadata_result(metadata),
            revision: file_revision(params.content.as_bytes()),
        })
    }

    pub(super) fn fs_create_file(&self, params: &Value) -> Result<Value, RpcError> {
        let params: FsCreateFileParams = decode(params)?;
        let metadata = self
            .file_system_service()?
            .create_file(&params.path, existing_behavior(params.existing))
            .map_err(file_system_error)?;
        result(&metadata_result(metadata))
    }

    pub(super) fn fs_rename(&self, params: &Value) -> Result<Value, RpcError> {
        let params: FsRenameParams = decode(params)?;
        self.file_system_service()?
            .rename(
                &params.source,
                &params.target,
                existing_behavior(params.existing),
            )
            .map_err(file_system_error)?;
        result(&())
    }

    pub(super) fn fs_delete(&self, params: &Value) -> Result<Value, RpcError> {
        let params: FsDeleteParams = decode(params)?;
        let missing = match params.missing {
            FsMissingTargetBehavior::Error => MissingTargetBehavior::Error,
            FsMissingTargetBehavior::Ignore => MissingTargetBehavior::Ignore,
        };
        let mode = match params.mode {
            FsDeleteMode::FileOrEmptyDirectory => FileDeleteMode::FileOrEmptyDirectory,
            FsDeleteMode::Recursive => FileDeleteMode::Recursive,
        };
        self.file_system_service()?
            .delete(&params.path, missing, mode)
            .map_err(file_system_error)?;
        result(&())
    }
}

fn existing_behavior(value: FsExistingTargetBehavior) -> ExistingTargetBehavior {
    match value {
        FsExistingTargetBehavior::Error => ExistingTargetBehavior::Error,
        FsExistingTargetBehavior::Overwrite => ExistingTargetBehavior::Overwrite,
        FsExistingTargetBehavior::Ignore => ExistingTargetBehavior::Ignore,
    }
}

fn metadata_result(metadata: FileMetadata) -> FsGetMetadataResult {
    FsGetMetadataResult {
        file_type: file_type(metadata.file_type),
        size_bytes: metadata.size_bytes,
        readonly: metadata.readonly,
        modified_at_millis: metadata.modified_at_millis,
    }
}

fn file_type(file_type: FileType) -> FsFileType {
    match file_type {
        FileType::Directory => FsFileType::Directory,
        FileType::File => FsFileType::File,
        FileType::SymbolicLink => FsFileType::SymbolicLink,
        FileType::Other => FsFileType::Other,
    }
}

fn file_system_error(error: FileSystemError) -> RpcError {
    match error {
        FileSystemError::NotFound(_) => {
            RpcError::new(-32043, AppServerErrorName::FileSystemNotFound)
        }
        FileSystemError::RevisionConflict(_) => {
            RpcError::new(-32042, AppServerErrorName::FileSystemRevisionConflict)
        }
        _ => RpcError::new(-32041, AppServerErrorName::FileSystemOperationFailed),
    }
}
