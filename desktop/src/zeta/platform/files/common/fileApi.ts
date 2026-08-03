import type { FsGetMetadataParams, FsGetMetadataResult, FsReadDirectoryParams, FsReadDirectoryResult, FsReadFileParams, FsReadFileResult, FsWriteFileParams, FsWriteFileResult } from "../../../../../generated/app-server/types.js";

export interface IFileApi {
  getMetadata(params: FsGetMetadataParams): Promise<FsGetMetadataResult>;
  readDirectory(params: FsReadDirectoryParams): Promise<FsReadDirectoryResult>;
  readFile(params: FsReadFileParams): Promise<FsReadFileResult>;
  writeFile(params: FsWriteFileParams): Promise<FsWriteFileResult>;
}
