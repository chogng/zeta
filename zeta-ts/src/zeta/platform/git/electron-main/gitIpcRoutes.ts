import { APP_SERVER_METHODS, type GitBranchSwitchParams, type GitChangeFileParams, type GitCommitChangesParams, type GitCommitFileParams, type GitCommitParams, type GitGraphParams, type GitPathsParams, type GitRepositoryParams } from "../../../../../generated/app-server/types.js";
import { VSBuffer } from "../../../base/common/buffer.js";
import type { AppServerSupervisor } from "../../app-server/electron-main/app-server-supervisor.js";
import { boundedPositiveInteger, record, string } from "../../ipc/electron-main/ipcValidation.js";
import type { IpcRoute } from "../../ipc/electron-main/trustedIpcRouter.js";
import { relativeWorkspacePath } from "../../workspace/electron-main/workspacePathValidation.js";

/** Exact-shape IPC routes for Git query and mutation operations. */
export function gitIpcRoutes(supervisor: AppServerSupervisor): readonly IpcRoute<unknown, unknown>[] {
	return [
		route({
			channel: "zeta:git:repositories",
			validate: emptyParams,
			invoke: () => supervisor.request(APP_SERVER_METHODS["git/repositories"], {}),
		}),
		route({
			channel: "zeta:git:status",
			validate: repositoryParams,
			invoke: (params) => supervisor.request(APP_SERVER_METHODS["git/status"], params),
		}),
		route({
			channel: "zeta:git:history",
			validate: repositoryParams,
			invoke: (params) => supervisor.request(APP_SERVER_METHODS["git/history"], params),
		}),
		route({
			channel: "zeta:git:branches",
			validate: repositoryParams,
			invoke: (params) => supervisor.request(APP_SERVER_METHODS["git/branch/list"], params),
		}),
		route({
			channel: "zeta:git:switch-branch",
			validate: branchSwitchParams,
			invoke: (params) => supervisor.request(APP_SERVER_METHODS["git/branch/switch"], params),
		}),
		route({
			channel: "zeta:git:graph",
			validate: graphParams,
			invoke: (params) => supervisor.request(APP_SERVER_METHODS["git/graph"], params),
		}),
		route({
			channel: "zeta:git:commit-changes",
			validate: commitChangesParams,
			invoke: (params) => supervisor.request(APP_SERVER_METHODS["git/commitChanges"], params),
		}),
		route({
			channel: "zeta:git:commit-file",
			validate: commitFileParams,
			invoke: (params) => supervisor.request(APP_SERVER_METHODS["git/commitFile"], params),
		}),
		route({
			channel: "zeta:git:change-file",
			validate: changeFileParams,
			invoke: (params) => supervisor.request(APP_SERVER_METHODS["git/changeFile"], params),
		}),
		route({
			channel: "zeta:git:stage",
			validate: gitPathsParams,
			invoke: (params) => supervisor.request(APP_SERVER_METHODS["git/stage"], params),
		}),
		route({
			channel: "zeta:git:unstage",
			validate: gitPathsParams,
			invoke: (params) => supervisor.request(APP_SERVER_METHODS["git/unstage"], params),
		}),
		route({
			channel: "zeta:git:discard-worktree",
			validate: gitPathsParams,
			invoke: (params) => supervisor.request(APP_SERVER_METHODS["git/discardWorktree"], params),
		}),
		route({
			channel: "zeta:git:commit",
			validate: gitCommitParams,
			invoke: (params) => supervisor.request(APP_SERVER_METHODS["git/commit"], params),
		}),
		route({
			channel: "zeta:git:fetch",
			validate: repositoryParams,
			invoke: (params) => supervisor.request(APP_SERVER_METHODS["git/fetch"], params),
		}),
		route({
			channel: "zeta:git:pull",
			validate: repositoryParams,
			invoke: (params) => supervisor.request(APP_SERVER_METHODS["git/pull"], params),
		}),
		route({
			channel: "zeta:git:push",
			validate: repositoryParams,
			invoke: (params) => supervisor.request(APP_SERVER_METHODS["git/push"], params),
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

function emptyParams(value: unknown): Record<string, never> {
	if (value === undefined) return {};
	return record(value, []) as Record<string, never>;
}

function gitPathsParams(value: unknown): GitPathsParams {
	const params = record(value, ["paths"], ["repositoryId"]);
	if (!Array.isArray(params.paths) || params.paths.length === 0 || params.paths.length > 5_000) {
		throw new Error("paths must contain between 1 and 5000 entries");
	}
	return {
		...(params.repositoryId === undefined ? {} : { repositoryId: gitRepositoryId(params.repositoryId) }),
		paths: params.paths.map((path, index) => {
			const resolved = relativeWorkspacePath(path);
			if (!resolved) throw new Error(`paths[${index}] must not be empty`);
			return resolved;
		}),
	};
}

function gitCommitParams(value: unknown): GitCommitParams {
	const params = record(value, ["message"], ["repositoryId"]);
	const message = string(params.message, "message");
	if (!message.trim() || message.includes("\0") || VSBuffer.fromString(message).byteLength > 65_536) {
		throw new Error("message must be non-empty, NUL-free, and no larger than 65536 UTF-8 bytes");
	}
	return { ...(params.repositoryId === undefined ? {} : { repositoryId: gitRepositoryId(params.repositoryId) }), message };
}

function repositoryParams(value: unknown): GitRepositoryParams {
	const params = record(value, [], ["repositoryId"]);
	return params.repositoryId === undefined ? {} : { repositoryId: gitRepositoryId(params.repositoryId) };
}

function branchSwitchParams(value: unknown): GitBranchSwitchParams {
	const params = record(value, ["name"], ["repositoryId"]);
	const name = string(params.name, "name");
	if (!name.trim() || name.includes("\0") || name.length > 1024) throw new Error("name must be non-empty, NUL-free, and no longer than 1024 characters");
	return { ...(params.repositoryId === undefined ? {} : { repositoryId: gitRepositoryId(params.repositoryId) }), name };
}

function graphParams(value: unknown): GitGraphParams {
	const params = record(value, ["limit"], ["cursor", "repositoryId"]);
	return {
		...(params.repositoryId === undefined ? {} : { repositoryId: gitRepositoryId(params.repositoryId) }),
		limit: boundedPositiveInteger(params.limit, "limit", 1000),
		...(params.cursor === undefined ? {} : { cursor: string(params.cursor, "cursor") }),
	};
}

function commitChangesParams(value: unknown): GitCommitChangesParams {
	const params = record(value, ["objectId"], ["repositoryId"]);
	return { ...(params.repositoryId === undefined ? {} : { repositoryId: gitRepositoryId(params.repositoryId) }), objectId: objectId(params.objectId) };
}

function commitFileParams(value: unknown): GitCommitFileParams {
	const params = record(value, ["objectId", "path"], ["repositoryId"]);
	const path = relativeWorkspacePath(params.path);
	if (!path) throw new Error("path must not be empty");
	return { ...(params.repositoryId === undefined ? {} : { repositoryId: gitRepositoryId(params.repositoryId) }), objectId: objectId(params.objectId), path };
}

function changeFileParams(value: unknown): GitChangeFileParams {
	const params = record(value, ["path", "comparison"], ["repositoryId"]);
	const path = relativeWorkspacePath(params.path);
	if (!path) throw new Error("path must not be empty");
	const comparison = string(params.comparison, "comparison");
	if (comparison !== "staged" && comparison !== "unstaged") throw new Error("comparison must be staged or unstaged");
	return { ...(params.repositoryId === undefined ? {} : { repositoryId: gitRepositoryId(params.repositoryId) }), path, comparison };
}

function gitRepositoryId(value: unknown): string {
	const id = string(value, "repositoryId");
	if (!/^repo_[0-9a-f]{64}$/u.test(id)) throw new Error("repositoryId must be a generated Git repository ID");
	return id;
}

function objectId(value: unknown): string {
	const objectId = string(value, "objectId");
	if (!/^[0-9a-fA-F]{40,64}$/.test(objectId)) throw new Error("objectId must be a 40-64 character hexadecimal hash");
	return objectId;
}
