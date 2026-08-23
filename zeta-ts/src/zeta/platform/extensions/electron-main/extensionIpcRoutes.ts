import { APP_SERVER_METHODS, type ExtensionListParams, type ExtensionResourceOpenParams } from "../../../../../generated/app-server/types.js";
import type { AppServerSupervisor } from "../../app-server/electron-main/app-server-supervisor.js";
import { nonEmptyString, nonNegativeInteger, record, string, stringEnum } from "../../ipc/electron-main/ipcValidation.js";
import type { IpcRoute } from "../../ipc/electron-main/trustedIpcRouter.js";

/** Exact-shape IPC routes for Rust-owned declarative extension resources. */
export function extensionIpcRoutes(supervisor: AppServerSupervisor): readonly IpcRoute<unknown, unknown>[] {
	return [
		route({
			channel: "zeta:extensions:list",
			validate: extensionListParams,
			invoke: params => supervisor.request(APP_SERVER_METHODS["extensions/list"], params),
		}),
		route({
			channel: "zeta:extensions:resource-open",
			validate: extensionResourceOpenParams,
			invoke: params => supervisor.request(APP_SERVER_METHODS["extensions/resource/open"], params),
		}),
	];
}

function route<P, R>(definition: IpcRoute<P, R>): IpcRoute<unknown, unknown> {
	return {
		channel: definition.channel,
		validate: definition.validate,
		invoke: params => definition.invoke(params as P),
	};
}

function extensionListParams(value: unknown): ExtensionListParams {
	const params = record(value, ["reload"]);
	return {
		reload: stringEnum(params.reload, "reload", ["cached", "refresh"] as const),
	};
}

function extensionResourceOpenParams(value: unknown): ExtensionResourceOpenParams {
	const params = record(value, ["extensionId", "generation", "path"]);
	return {
		generation: nonNegativeInteger(params.generation, "generation"),
		extensionId: nonEmptyString(params.extensionId, "extensionId"),
		path: string(params.path, "path"),
	};
}
