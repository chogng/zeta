import type { FsGetMetadataResult, FsReadDirectoryResult, FsReadFileResult } from "../../../../../generated/app-server/types.js";
import { invoke } from "../../ipc/electron-browser/rendererIpc.js";
import type { IFileApi } from "../common/fileApi.js";

export function createFileApi(): IFileApi {
  return {
    getMetadata: (params) => invoke<FsGetMetadataResult>("zeta:fs:get-metadata", params),
    readDirectory: (params) => invoke<FsReadDirectoryResult>("zeta:fs:read-directory", params),
    readFile: (params) => invoke<FsReadFileResult>("zeta:fs:read-file", params),
  };
}
