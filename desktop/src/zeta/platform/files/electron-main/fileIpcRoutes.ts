import { APP_SERVER_METHODS, type FsGetMetadataParams, type FsReadDirectoryParams, type FsReadFileParams, type FsWriteFileParams } from "../../../../../generated/app-server/types.js";
import type { AppServerSupervisor } from "../../app-server/electron-main/app-server-supervisor.js";
import { record, string } from "../../ipc/electron-main/ipcValidation.js";
import type { IpcRoute } from "../../ipc/electron-main/trustedIpcRouter.js";
import { relativeWorkspacePath } from "../../workspace/electron-main/workspacePathValidation.js";

/** Exact-shape IPC routes for workspace file operations. */
export function fileIpcRoutes(supervisor: AppServerSupervisor): readonly IpcRoute<unknown, unknown>[] {
  return [
    route({
      channel: "zeta:fs:get-metadata",
      validate: fsGetMetadataParams,
      invoke: (params) => supervisor.request(APP_SERVER_METHODS["fs/getMetadata"], params),
    }),
    route({
      channel: "zeta:fs:read-directory",
      validate: fsReadDirectoryParams,
      invoke: (params) => supervisor.request(APP_SERVER_METHODS["fs/readDirectory"], params),
    }),
    route({
      channel: "zeta:fs:read-file",
      validate: fsReadFileParams,
      invoke: (params) => supervisor.request(APP_SERVER_METHODS["fs/readFile"], params),
    }),
    route({
      channel: "zeta:fs:write-file",
      validate: fsWriteFileParams,
      invoke: (params) => supervisor.request(APP_SERVER_METHODS["fs/writeFile"], params),
    }),
  ];
}

function route<P, R>(definition: IpcRoute<P, R>): IpcRoute<unknown, unknown> {
  return {
    channel: definition.channel,
    validate: definition.validate,
    invoke: (params) => definition.invoke(params as P),
  };
}

function fsGetMetadataParams(value: unknown): FsGetMetadataParams {
  const params = record(value, ["path"]);
  return { path: relativeWorkspacePath(params.path) };
}

function fsReadDirectoryParams(value: unknown): FsReadDirectoryParams {
  return fsGetMetadataParams(value);
}

function fsReadFileParams(value: unknown): FsReadFileParams {
  return fsGetMetadataParams(value);
}

function fsWriteFileParams(value: unknown): FsWriteFileParams {
  const params = record(value, ["path", "content"]);
  return {
    path: relativeWorkspacePath(params.path),
    content: string(params.content, "content"),
  };
}
