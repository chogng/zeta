import { APP_SERVER_METHODS, type ExtensionHostInvokeCancelParams, type ExtensionHostInvokeReadParams, type ExtensionHostInvokeStartParams, type ExtensionHostReconcileParams } from "../../../../../generated/app-server/types.js";
import { normalizeExtensionHostInvocationRequest, type ExtensionHostInvocationRequest } from "../common/extensionHostApi.js";
import type { AppServerSupervisor } from "../../app-server/electron-main/app-server-supervisor.js";
import { nonEmptyString, record, stringEnum } from "../../ipc/electron-main/ipcValidation.js";
import type { IpcRoute } from "../../ipc/electron-main/trustedIpcRouter.js";

/** Exact-shape IPC routes for the App Server-owned Extension Host fleet. */
export function extensionHostIpcRoutes(supervisor: AppServerSupervisor): readonly IpcRoute<unknown, unknown>[] {
	return [
		route({ channel: "zeta:extension-host:available", validate: emptyParams, invoke: () => supervisor.capabilities?.extensionHost === true }),
		route({ channel: "zeta:extension-host:list", validate: emptyParams, invoke: () => supervisor.request(APP_SERVER_METHODS["extensionHost/list"], {}) }),
		route({ channel: "zeta:extension-host:reconcile", validate: reconcileParams, invoke: params => supervisor.request(APP_SERVER_METHODS["extensionHost/reconcile"], params) }),
		route({ channel: "zeta:extension-host:invoke-start", validate: invokeStartParams, invoke: params => supervisor.request(APP_SERVER_METHODS["extensionHost/invoke/start"], params) }),
		route({ channel: "zeta:extension-host:invoke-read", validate: invokeReadParams, invoke: params => supervisor.request(APP_SERVER_METHODS["extensionHost/invoke/read"], params) }),
		route({ channel: "zeta:extension-host:invoke-cancel", validate: invokeCancelParams, invoke: params => supervisor.request(APP_SERVER_METHODS["extensionHost/invoke/cancel"], params) }),
	];
}

function route<P, R>(definition: IpcRoute<P, R>): IpcRoute<unknown, unknown> {
	return { channel: definition.channel, validate: definition.validate, invoke: params => definition.invoke(params as P) };
}

function emptyParams(value: unknown): Record<string, never> {
	if (value === undefined) return {};
	return record(value, []) as Record<string, never>;
}

function reconcileParams(value: unknown): ExtensionHostReconcileParams {
	const params = record(value, ["mode"]);
	return { mode: stringEnum(params.mode, "mode", ["refresh", "restartFailed"] as const) };
}

function invokeStartParams(value: unknown): ExtensionHostInvokeStartParams {
	return normalizeExtensionHostInvocationRequest(value as ExtensionHostInvocationRequest);
}

function invokeReadParams(value: unknown): ExtensionHostInvokeReadParams {
	const params = record(value, ["invocationId"]);
	return { invocationId: boundedId(params.invocationId) };
}

function invokeCancelParams(value: unknown): ExtensionHostInvokeCancelParams {
	return invokeReadParams(value);
}

function boundedId(value: unknown): string {
	const id = nonEmptyString(value, "invocationId");
	if (id.length > 256 || id.includes("\0")) throw new Error("invocationId is invalid");
	return id;
}
