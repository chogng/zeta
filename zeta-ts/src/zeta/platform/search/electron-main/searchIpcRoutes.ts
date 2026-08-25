import { APP_SERVER_METHODS, type WorkspaceSearchCancelParams, type WorkspaceSearchReadParams, type WorkspaceSearchStartParams } from "../../../../../generated/app-server/types.js";
import { VSBuffer } from "../../../base/common/buffer.js";
import type { AppServerSupervisor } from "../../app-server/electron-main/app-server-supervisor.js";
import { boundedPositiveInteger, nonEmptyString, nonNegativeInteger, record, stringEnum } from "../../ipc/electron-main/ipcValidation.js";
import type { IpcRoute } from "../../ipc/electron-main/trustedIpcRouter.js";

/** Exact-shape IPC routes for workspace search jobs. */
export function searchIpcRoutes(supervisor: AppServerSupervisor): readonly IpcRoute<unknown, unknown>[] {
	return [
		route({
			channel: "zeta:workspace-search:start",
			validate: workspaceSearchStartParams,
			invoke: (params) => supervisor.request(APP_SERVER_METHODS["workspace/search/start"], params),
		}),
		route({
			channel: "zeta:workspace-search:read",
			validate: workspaceSearchReadParams,
			invoke: (params) => supervisor.request(APP_SERVER_METHODS["workspace/search/read"], params),
		}),
		route({
			channel: "zeta:workspace-search:cancel",
			validate: workspaceSearchCancelParams,
			invoke: (params) => supervisor.request(APP_SERVER_METHODS["workspace/search/cancel"], params),
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

function workspaceSearchStartParams(value: unknown): WorkspaceSearchStartParams {
	const params = record(value, ["query", "patternKind", "caseSensitivity", "includePatterns", "excludePatterns", "maxResults"], ["workspaceFolderId"]);
	const query = nonEmptyString(params.query, "query");
	if (VSBuffer.fromString(query).byteLength > 16_384) {
		throw new Error("query must not exceed 16384 UTF-8 bytes");
	}
	return {
		...workspaceFolder(params.workspaceFolderId),
		query,
		patternKind: stringEnum(params.patternKind, "patternKind", ["literal", "regex"] as const),
		caseSensitivity: stringEnum(params.caseSensitivity, "caseSensitivity", ["smart", "sensitive", "insensitive"] as const),
		includePatterns: searchPatterns(params.includePatterns, "includePatterns"),
		excludePatterns: searchPatterns(params.excludePatterns, "excludePatterns"),
		maxResults: boundedPositiveInteger(params.maxResults, "maxResults", 5_000),
	};
}

function workspaceSearchReadParams(value: unknown): WorkspaceSearchReadParams {
	const params = record(value, ["searchId", "afterMatch", "maxMatches"], ["workspaceFolderId"]);
	return {
		...workspaceFolder(params.workspaceFolderId),
		searchId: nonEmptyString(params.searchId, "searchId"),
		afterMatch: nonNegativeInteger(params.afterMatch, "afterMatch"),
		maxMatches: boundedPositiveInteger(params.maxMatches, "maxMatches", 200),
	};
}

function workspaceSearchCancelParams(value: unknown): WorkspaceSearchCancelParams {
	const params = record(value, ["searchId"], ["workspaceFolderId"]);
	return { ...workspaceFolder(params.workspaceFolderId), searchId: nonEmptyString(params.searchId, "searchId") };
}

function workspaceFolder(value: unknown): { readonly workspaceFolderId?: string } {
	return value === undefined ? {} : { workspaceFolderId: nonEmptyString(value, "workspaceFolderId") };
}

function searchPatterns(value: unknown, field: string): string[] {
	if (!Array.isArray(value) || value.length > 64) {
		throw new Error(`${field} must be an array with at most 64 entries`);
	}
	return value.map((entry, index) => {
		const pattern = nonEmptyString(entry, `${field}[${index}]`);
		if (VSBuffer.fromString(pattern).byteLength > 1_024 || pattern.includes("\0") || pattern.startsWith("!") || pattern.startsWith("/") || /^[A-Za-z]:[\\/]/.test(pattern) || pattern.replaceAll("\\", "/").split("/").includes("..")) {
			throw new Error(`${field}[${index}] must be a workspace-relative glob`);
		}
		return pattern;
	});
}
