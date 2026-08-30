import { APP_SERVER_METHODS, type TerminalCloseParams, type TerminalCreateParams, type TerminalReadParams, type TerminalResizeParams, type TerminalWriteParams } from "../../../../../generated/app-server/types.js";
import { VSBuffer } from "../../../base/common/buffer.js";
import type { AppServerSupervisor } from "../../app-server/electron-main/app-server-supervisor.js";
import { boundedPositiveInteger, nonEmptyString, nonNegativeInteger, record, string } from "../../ipc/electron-main/ipcValidation.js";
import type { IpcRoute } from "../../ipc/electron-main/trustedIpcRouter.js";
import type { ReconnectableTerminalMainService } from "./reconnectableTerminalMainService.js";

/** Exact-shape IPC routes for terminal process operations. */
export function terminalIpcRoutes(supervisor: AppServerSupervisor, reconnectable?: ReconnectableTerminalMainService): readonly IpcRoute<unknown, unknown>[] {
	return [
		route({
			channel: "zeta:terminal:profile-list",
			validate: emptyParams,
			invoke: () => supervisor.request(APP_SERVER_METHODS["terminal/profile/list"], {}),
		}),
		route({
			channel: "zeta:terminal:create",
			validate: terminalCreateParams,
			invoke: async (params) => {
				if (reconnectable) return reconnectable.create(params);
				const created = await supervisor.request(APP_SERVER_METHODS["terminal/create"], params);
				return { terminalId: created.terminalId, profile: created.profile, connectionPersistence: "connectionOwned" as const };
			},
		}),
		route({
			channel: "zeta:terminal:write",
			validate: terminalWriteParams,
			invoke: (params) => reconnectable ? reconnectable.write(params) : supervisor.request(APP_SERVER_METHODS["terminal/write"], params),
		}),
		route({
			channel: "zeta:terminal:resize",
			validate: terminalResizeParams,
			invoke: (params) => reconnectable ? reconnectable.resize(params) : supervisor.request(APP_SERVER_METHODS["terminal/resize"], params),
		}),
		route({
			channel: "zeta:terminal:read",
			validate: terminalReadParams,
			invoke: (params) => reconnectable ? reconnectable.read(params) : supervisor.request(APP_SERVER_METHODS["terminal/read"], params),
		}),
		route({
			channel: "zeta:terminal:close",
			validate: terminalCloseParams,
			invoke: (params) => reconnectable ? reconnectable.close(params) : supervisor.request(APP_SERVER_METHODS["terminal/close"], params),
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

function terminalCreateParams(value: unknown): TerminalCreateParams {
	const params = record(value, ["rows", "cols", "profile", "lifecycle"], ["dirId"]);
	const lifecycle = record(params.lifecycle, ["type"]);
	if (lifecycle.type !== "connectionOwned") {
		throw new Error("Desktop renderer terminals must be connectionOwned");
	}
	return {
		...workspaceFolder(params.dirId),
		rows: boundedPositiveInteger(params.rows, "rows", 512),
		cols: boundedPositiveInteger(params.cols, "cols", 512),
		profile: terminalProfileSelection(params.profile),
		lifecycle: { type: "connectionOwned" },
	};
}

function terminalProfileSelection(value: unknown): TerminalCreateParams["profile"] {
	const profile = value as Record<string, unknown>;
	if (typeof profile !== "object" || profile === null || Array.isArray(profile)) {
		throw new Error("profile must be an object");
	}
	if (profile.type === "default") {
		record(profile, ["type"]);
		return { type: "default" };
	}
	if (profile.type === "profile") {
		const selected = record(profile, ["type", "profileId"]);
		return {
			type: "profile",
			profileId: nonEmptyString(selected.profileId, "profile.profileId"),
		};
	}
	throw new Error("profile.type must be default or profile");
}

function terminalWriteParams(value: unknown): TerminalWriteParams {
	const params = record(value, ["terminalId", "data"], ["dirId"]);
	const data = string(params.data, "data");
	if (data.length === 0) throw new Error("data must not be empty");
	if (VSBuffer.fromString(data).byteLength > 65_536) {
		throw new Error("data must not exceed 65536 UTF-8 bytes");
	}
	return {
		...workspaceFolder(params.dirId),
		terminalId: nonEmptyString(params.terminalId, "terminalId"),
		data,
	};
}

function terminalResizeParams(value: unknown): TerminalResizeParams {
	const params = record(value, ["terminalId", "rows", "cols"], ["dirId"]);
	return {
		...workspaceFolder(params.dirId),
		terminalId: nonEmptyString(params.terminalId, "terminalId"),
		rows: boundedPositiveInteger(params.rows, "rows", 512),
		cols: boundedPositiveInteger(params.cols, "cols", 512),
	};
}

function terminalReadParams(value: unknown): TerminalReadParams {
	const params = record(value, ["terminalId", "afterSequence", "afterCommandSequence", "maxChunks"], ["dirId"]);
	return {
		...workspaceFolder(params.dirId),
		terminalId: nonEmptyString(params.terminalId, "terminalId"),
		afterSequence: nonNegativeInteger(params.afterSequence, "afterSequence"),
		afterCommandSequence: nonNegativeInteger(params.afterCommandSequence, "afterCommandSequence"),
		maxChunks: boundedPositiveInteger(params.maxChunks, "maxChunks", 128),
	};
}

function terminalCloseParams(value: unknown): TerminalCloseParams {
	const params = record(value, ["terminalId"], ["dirId"]);
	return { ...workspaceFolder(params.dirId), terminalId: nonEmptyString(params.terminalId, "terminalId") };
}

function workspaceFolder(value: unknown): { readonly dirId?: string } {
	return value === undefined ? {} : { dirId: nonEmptyString(value, "dirId") };
}
