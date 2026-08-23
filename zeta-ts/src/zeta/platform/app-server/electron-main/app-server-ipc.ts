import { APP_SERVER_METHODS, type ResourceMetadataParams, type ResourceReadParams, type ResourceReleaseParams } from "../../../../../generated/app-server/types.js";
import type { AppServerSupervisor } from "./app-server-supervisor.js";
import { nonEmptyString, nonNegativeInteger, positiveInteger, record } from "../../ipc/electron-main/ipcValidation.js";
import type { IpcRoute } from "../../ipc/electron-main/trustedIpcRouter.js";

const MAX_RESOURCE_READ_BYTES = 262_144;

/** IPC routes owned by the App Server connection and resource infrastructure. */
export function appServerIpcRoutes(supervisor: AppServerSupervisor): readonly IpcRoute<unknown, unknown>[] {
	return [
		route({
			channel: "zeta:app-server:state",
			validate: emptyParams,
			invoke: () => supervisor.state,
		}),
		route({
			channel: "zeta:app-server:slash-commands",
			validate: emptyParams,
			invoke: () => supervisor.slashCommands,
		}),
		route({
			channel: "zeta:resource:metadata",
			validate: resourceMetadataParams,
			invoke: (params) => supervisor.request(APP_SERVER_METHODS["resource/metadata"], params),
		}),
		route({
			channel: "zeta:resource:read",
			validate: resourceReadParams,
			invoke: (params) => supervisor.request(APP_SERVER_METHODS["resource/read"], params),
		}),
		route({
			channel: "zeta:resource:release",
			validate: resourceReleaseParams,
			invoke: (params) => supervisor.request(APP_SERVER_METHODS["resource/release"], params),
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

function resourceMetadataParams(value: unknown): ResourceMetadataParams {
	const params = record(value, ["resourceId"]);
	return { resourceId: nonEmptyString(params.resourceId, "resourceId") };
}

function resourceReleaseParams(value: unknown): ResourceReleaseParams {
	return resourceMetadataParams(value);
}

function resourceReadParams(value: unknown): ResourceReadParams {
	const params = record(value, ["resourceId", "offset", "maxBytes"]);
	const maxBytes = positiveInteger(params.maxBytes, "maxBytes");
	if (maxBytes > MAX_RESOURCE_READ_BYTES) {
		throw new Error(`maxBytes must not exceed ${MAX_RESOURCE_READ_BYTES}`);
	}
	return {
		resourceId: nonEmptyString(params.resourceId, "resourceId"),
		offset: nonNegativeInteger(params.offset, "offset"),
		maxBytes,
	};
}
