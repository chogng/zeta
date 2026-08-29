import {
	APP_SERVER_METHODS,
	type TurnChangesCommitParams,
	type TurnChangesDiscardThreadParams,
	type TurnChangesListParams,
	type TurnChangesMutationParams,
	type TurnChangesReadFileParams,
	type TurnChangesReadParams,
	type TurnChangesUpdateDraftParams,
} from "../../../../../generated/app-server/types.js";
import type { AppServerSupervisor } from "../../app-server/electron-main/app-server-supervisor.js";
import { boolean, nonEmptyString, nonNegativeInteger, record, string } from "../../ipc/electron-main/ipcValidation.js";
import type { IpcRoute } from "../../ipc/electron-main/trustedIpcRouter.js";
import { relativeWorkspacePath } from "../../workspace/electron-main/workspacePathValidation.js";

/** Exact-shape IPC routes for Turn change ledger queries and mutations. */
export function turnChangesIpcRoutes(supervisor: AppServerSupervisor): readonly IpcRoute<unknown, unknown>[] {
	return [
		route({ channel: "zeta:turn-changes:list", validate: listParams, invoke: (params) => supervisor.request(APP_SERVER_METHODS["turnChanges/list"], params) }),
		route({ channel: "zeta:turn-changes:read", validate: readParams, invoke: (params) => supervisor.request(APP_SERVER_METHODS["turnChanges/read"], params) }),
		route({ channel: "zeta:turn-changes:read-file", validate: readFileParams, invoke: (params) => supervisor.request(APP_SERVER_METHODS["turnChanges/readFile"], params) }),
		route({ channel: "zeta:turn-changes:generate-message", validate: mutationParams, invoke: (params) => supervisor.request(APP_SERVER_METHODS["turnChanges/generateMessage"], params) }),
		route({ channel: "zeta:turn-changes:update-draft", validate: updateDraftParams, invoke: (params) => supervisor.request(APP_SERVER_METHODS["turnChanges/updateDraft"], params) }),
		route({ channel: "zeta:turn-changes:commit", validate: commitParams, invoke: (params) => supervisor.request(APP_SERVER_METHODS["turnChanges/commit"], params) }),
		route({ channel: "zeta:turn-changes:discard-thread", validate: discardThreadParams, invoke: (params) => supervisor.request(APP_SERVER_METHODS["turnChanges/discardThread"], params) }),
	];
}

function route<P, R>(definition: IpcRoute<P, R>): IpcRoute<unknown, unknown> {
	return { channel: definition.channel, validate: definition.validate, invoke: (params) => definition.invoke(params as P) };
}

function owner(value: unknown): { readonly sessionId: string; readonly threadId: string } {
	const params = record(value, ["sessionId", "threadId"]);
	return { sessionId: nonEmptyString(params.sessionId, "sessionId"), threadId: nonEmptyString(params.threadId, "threadId") };
}

function listParams(value: unknown): TurnChangesListParams {
	return owner(value);
}

function readParams(value: unknown): TurnChangesReadParams {
	const params = record(value, ["sessionId", "threadId", "changeSetId"]);
	return { ...owner(params), changeSetId: nonEmptyString(params.changeSetId, "changeSetId") };
}

function readFileParams(value: unknown): TurnChangesReadFileParams {
	const params = record(value, ["sessionId", "threadId", "changeSetId", "path"]);
	const path = relativeWorkspacePath(params.path);
	if (!path) throw new Error("path must not be empty");
	return { ...owner({ sessionId: params.sessionId, threadId: params.threadId }), changeSetId: nonEmptyString(params.changeSetId, "changeSetId"), path };
}

function mutationParams(value: unknown): TurnChangesMutationParams {
	const params = record(value, ["commandId", "sessionId", "threadId", "changeSetId", "expectedRevision"]);
	return {
		commandId: nonEmptyString(params.commandId, "commandId"),
		...owner({ sessionId: params.sessionId, threadId: params.threadId }),
		changeSetId: nonEmptyString(params.changeSetId, "changeSetId"),
		expectedRevision: nonNegativeInteger(params.expectedRevision, "expectedRevision"),
	};
}

function updateDraftParams(value: unknown): TurnChangesUpdateDraftParams {
	const params = record(value, ["commandId", "sessionId", "threadId", "changeSetId", "expectedRevision", "message"]);
	const message = string(params.message, "message");
	if (!message.trim() || message.includes("\0") || new TextEncoder().encode(message).byteLength > 65_536) {
		throw new Error("message must be non-empty, NUL-free, and no larger than 65536 UTF-8 bytes");
	}
	return { ...mutationParams({ commandId: params.commandId, sessionId: params.sessionId, threadId: params.threadId, changeSetId: params.changeSetId, expectedRevision: params.expectedRevision }), message };
}

function commitParams(value: unknown): TurnChangesCommitParams {
	const params = record(value, ["commandId", "sessionId", "threadId", "changeSetIds", "expectedRevision"]);
	if (!Array.isArray(params.changeSetIds) || params.changeSetIds.length !== 1) throw new Error("changeSetIds must contain exactly one ChangeSet");
	return {
		commandId: nonEmptyString(params.commandId, "commandId"),
		...owner({ sessionId: params.sessionId, threadId: params.threadId }),
		changeSetIds: [nonEmptyString(params.changeSetIds[0], "changeSetIds[0]")],
		expectedRevision: nonNegativeInteger(params.expectedRevision, "expectedRevision"),
	};
}

function discardThreadParams(value: unknown): TurnChangesDiscardThreadParams {
	const params = record(value, ["commandId", "sessionId", "threadId", "expectedRevision", "confirmed"]);
	return {
		commandId: nonEmptyString(params.commandId, "commandId"),
		...owner({ sessionId: params.sessionId, threadId: params.threadId }),
		expectedRevision: nonNegativeInteger(params.expectedRevision, "expectedRevision"),
		confirmed: boolean(params.confirmed, "confirmed"),
	};
}
