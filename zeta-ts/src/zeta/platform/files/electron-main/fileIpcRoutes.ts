import { APP_SERVER_METHODS, type FsCreateFileParams, type FsDeleteParams, type FsGetMetadataParams, type FsReadBinaryFileParams, type FsReadDirectoryParams, type FsReadFileParams, type FsRenameParams, type FsWriteFileParams } from "../../../../../generated/app-server/types.js";
import type { AppServerSupervisor } from "../../app-server/electron-main/app-server-supervisor.js";
import { record, string, stringEnum } from "../../ipc/electron-main/ipcValidation.js";
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
			channel: "zeta:fs:read-binary-file",
			validate: fsReadBinaryFileParams,
			invoke: (params) => supervisor.request(APP_SERVER_METHODS["fs/readBinaryFile"], params),
		}),
		route({
			channel: "zeta:fs:write-file",
			validate: fsWriteFileParams,
			invoke: (params) => supervisor.request(APP_SERVER_METHODS["fs/writeFile"], params),
		}),
		route({ channel: "zeta:fs:create-file", validate: fsCreateFileParams, invoke: params => supervisor.request(APP_SERVER_METHODS["fs/createFile"], params) }),
		route({ channel: "zeta:fs:rename", validate: fsRenameParams, invoke: params => supervisor.request(APP_SERVER_METHODS["fs/rename"], params) }),
		route({ channel: "zeta:fs:delete", validate: fsDeleteParams, invoke: params => supervisor.request(APP_SERVER_METHODS["fs/delete"], params) }),
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
	const params = record(value, ["path"], ["dirId"]);
	return { ...workspaceFolder(params.dirId), path: relativeWorkspacePath(params.path) };
}

function fsReadDirectoryParams(value: unknown): FsReadDirectoryParams {
	return fsGetMetadataParams(value);
}

function fsReadFileParams(value: unknown): FsReadFileParams {
	return fsGetMetadataParams(value);
}

function fsReadBinaryFileParams(value: unknown): FsReadBinaryFileParams {
	return fsGetMetadataParams(value);
}

function fsWriteFileParams(value: unknown): FsWriteFileParams {
	const params = record(value, ["path", "content"], ["expectedRevision", "dirId"]);
	return {
		...workspaceFolder(params.dirId),
		path: relativeWorkspacePath(params.path),
		content: string(params.content, "content"),
		...(params.expectedRevision === undefined ? {} : { expectedRevision: string(params.expectedRevision, "expectedRevision") }),
	};
}

function fsCreateFileParams(value: unknown): FsCreateFileParams {
	const params = record(value, ["path", "existing"], ["dirId"]);
	return { ...workspaceFolder(params.dirId), path: relativeWorkspacePath(params.path), existing: stringEnum(params.existing, "existing", ["error", "overwrite", "ignore"] as const) };
}

function fsRenameParams(value: unknown): FsRenameParams {
	const params = record(value, ["source", "target", "existing"], ["dirId"]);
	return { ...workspaceFolder(params.dirId), source: relativeWorkspacePath(params.source), target: relativeWorkspacePath(params.target), existing: stringEnum(params.existing, "existing", ["error", "overwrite", "ignore"] as const) };
}

function fsDeleteParams(value: unknown): FsDeleteParams {
	const params = record(value, ["path", "missing", "mode"], ["dirId"]);
	return { ...workspaceFolder(params.dirId), path: relativeWorkspacePath(params.path), missing: stringEnum(params.missing, "missing", ["error", "ignore"] as const), mode: stringEnum(params.mode, "mode", ["fileOrEmptyDirectory", "recursive"] as const) };
}

function workspaceFolder(value: unknown): { readonly dirId?: string } {
	if (value === undefined) return {};
	const dirId = string(value, "dirId");
	if (!dirId.trim()) throw new Error("dirId must be non-empty");
	return { dirId };
}
