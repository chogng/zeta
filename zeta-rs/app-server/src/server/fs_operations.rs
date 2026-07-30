use super::{AppServer, RpcError, decode, result};
use serde_json::Value;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_app_server_protocol::protocol::fs::{
    FsFileType, FsGetMetadataParams, FsGetMetadataResult, FsReadDirectoryEntry,
    FsReadDirectoryParams, FsReadDirectoryResult, FsReadFileParams, FsReadFileResult,
    FsWriteFileParams, FsWriteFileResult,
};
use zeta_file_system::{FileMetadata, FileSystemError, FileType};

const MAX_EDITOR_FILE_BYTES: usize = 10 * 1024 * 1024;

impl AppServer {
    pub(super) fn fs_get_metadata(&self, params: &Value) -> Result<Value, RpcError> {
        let params: FsGetMetadataParams = decode(params)?;
        let metadata = self
            .file_system()?
            .get_metadata(&params.path)
            .map_err(file_system_error)?;
        result(&metadata_result(metadata))
    }

    pub(super) fn fs_read_directory(&self, params: &Value) -> Result<Value, RpcError> {
        let params: FsReadDirectoryParams = decode(params)?;
        let entries = self
            .file_system()?
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
        let bytes = self
            .file_system()?
            .read_file(&params.path, MAX_EDITOR_FILE_BYTES)
            .map_err(file_system_error)?;
        let content = String::from_utf8(bytes).map_err(|_| {
            file_system_error(FileSystemError::Io("file is not valid UTF-8".into()))
        })?;
        result(&FsReadFileResult { content })
    }

    pub(super) fn fs_write_file(&self, params: &Value) -> Result<Value, RpcError> {
        let params: FsWriteFileParams = decode(params)?;
        let metadata = self
            .file_system()?
            .write_file(
                &params.path,
                params.content.as_bytes(),
                MAX_EDITOR_FILE_BYTES,
            )
            .map_err(file_system_error)?;
        result(&FsWriteFileResult {
            metadata: metadata_result(metadata),
        })
    }

    fn file_system(&self) -> Result<&dyn zeta_file_system::WorkspaceFileSystem, RpcError> {
        self.file_system
            .as_deref()
            .ok_or_else(|| RpcError::new(-32040, AppServerErrorName::FileSystemUnavailable))
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

fn file_system_error(_error: FileSystemError) -> RpcError {
    RpcError::new(-32041, AppServerErrorName::FileSystemOperationFailed)
}
