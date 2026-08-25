import { APP_SERVER_METHODS, type DebugAdapterCloseParams, type DebugAdapterReadParams, type DebugAdapterSendParams, type DebugAdapterStartParams } from "../../../../../generated/app-server/types.js";
import { VSBuffer } from "../../../base/common/buffer.js";
import { type AppServerSupervisor } from "../../app-server/electron-main/app-server-supervisor.js";
import { boundedPositiveInteger, nonEmptyString, nonNegativeInteger, record, string } from "../../ipc/electron-main/ipcValidation.js";
import { type IpcRoute } from "../../ipc/electron-main/trustedIpcRouter.js";

const MAX_ARGUMENTS = 128;
const MAX_ARGUMENT_BYTES = 32_768;
const MAX_MESSAGE_BYTES = 4 * 1024 * 1024;

/** Exact-shape IPC routes for generic DAP process operations. */
export function debugAdapterIpcRoutes(supervisor: AppServerSupervisor): readonly IpcRoute<unknown, unknown>[] {
	return [
		route({ channel: "zeta:debug-adapter:start", validate: startParams, invoke: params => supervisor.request(APP_SERVER_METHODS["debug/adapter/start"], params) }),
		route({ channel: "zeta:debug-adapter:send", validate: sendParams, invoke: params => supervisor.request(APP_SERVER_METHODS["debug/adapter/send"], params) }),
		route({ channel: "zeta:debug-adapter:read", validate: readParams, invoke: params => supervisor.request(APP_SERVER_METHODS["debug/adapter/read"], params) }),
		route({ channel: "zeta:debug-adapter:close", validate: closeParams, invoke: params => supervisor.request(APP_SERVER_METHODS["debug/adapter/close"], params) }),
	];
}

function startParams(value: unknown): DebugAdapterStartParams {
	const params = record(value, ["program", "arguments"], ["workspaceFolderId"]);
	if (!Array.isArray(params.arguments) || params.arguments.length > MAX_ARGUMENTS) throw new Error("arguments must be a bounded array");
	const argumentsList = params.arguments.map((argument, index) => string(argument, `arguments[${index}]`));
	if (argumentsList.reduce((bytes, argument) => bytes + VSBuffer.fromString(argument).byteLength, 0) > MAX_ARGUMENT_BYTES) throw new Error("arguments exceed the supported size");
	return { ...workspaceFolder(params.workspaceFolderId), program: nonEmptyString(params.program, "program"), arguments: argumentsList };
}

function sendParams(value: unknown): DebugAdapterSendParams {
	const params = record(value, ["sessionId", "message"], ["workspaceFolderId"]);
	const message = jsonValue(params.message, "message");
	if (VSBuffer.fromString(JSON.stringify(message)).byteLength > MAX_MESSAGE_BYTES) throw new Error("message exceeds the supported size");
	return { ...workspaceFolder(params.workspaceFolderId), sessionId: nonEmptyString(params.sessionId, "sessionId"), message };
}

function readParams(value: unknown): DebugAdapterReadParams {
	const params = record(value, ["sessionId", "afterSequence", "maxMessages"], ["workspaceFolderId"]);
	return { ...workspaceFolder(params.workspaceFolderId), sessionId: nonEmptyString(params.sessionId, "sessionId"), afterSequence: nonNegativeInteger(params.afterSequence, "afterSequence"), maxMessages: boundedPositiveInteger(params.maxMessages, "maxMessages", 128) };
}

function closeParams(value: unknown): DebugAdapterCloseParams {
	const params = record(value, ["sessionId"], ["workspaceFolderId"]);
	return { ...workspaceFolder(params.workspaceFolderId), sessionId: nonEmptyString(params.sessionId, "sessionId") };
}

function workspaceFolder(value: unknown): { readonly workspaceFolderId?: string } {
	return value === undefined ? {} : { workspaceFolderId: nonEmptyString(value, "workspaceFolderId") };
}

function jsonValue(value: unknown, field: string, depth = 0): unknown {
	if (depth > 64) throw new Error(`${field} exceeds the supported nesting depth`);
	if (value === null || typeof value === "string" || typeof value === "boolean" || (typeof value === "number" && Number.isFinite(value))) return value;
	if (Array.isArray(value)) return value.map((item, index) => jsonValue(item, `${field}[${index}]`, depth + 1));
	if (typeof value === "object") return Object.fromEntries(Object.entries(value as Record<string, unknown>).map(([key, item]) => [key, jsonValue(item, `${field}.${key}`, depth + 1)]));
	throw new Error(`${field} must be JSON-compatible`);
}

function route<P, R>(definition: IpcRoute<P, R>): IpcRoute<unknown, unknown> {
	return { channel: definition.channel, validate: definition.validate, invoke: params => definition.invoke(params as P) };
}
