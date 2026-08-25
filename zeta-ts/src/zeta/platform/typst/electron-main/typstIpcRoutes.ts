import { APP_SERVER_METHODS, type TypstCompileParams } from "../../../../../generated/app-server/types.js";
import { VSBuffer } from "../../../base/common/buffer.js";
import type { AppServerSupervisor } from "../../app-server/electron-main/app-server-supervisor.js";
import { record, string } from "../../ipc/electron-main/ipcValidation.js";
import type { IpcRoute } from "../../ipc/electron-main/trustedIpcRouter.js";

const MAX_TYPST_SOURCE_BYTES = 1024 * 1024;

/** Exact-shape IPC routes for Typst document compilation. */
export function typstIpcRoutes(supervisor: AppServerSupervisor): readonly IpcRoute<unknown, unknown>[] {
	return [
		route({
			channel: "zeta:typst:compile",
			validate: typstCompileParams,
			invoke: (params) => supervisor.request(APP_SERVER_METHODS["document/typst/compile"], params),
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

function typstCompileParams(value: unknown): TypstCompileParams {
	const params = record(value, ["source"]);
	const source = string(params.source, "source");
	if (VSBuffer.fromString(source).byteLength > MAX_TYPST_SOURCE_BYTES) {
		throw new Error(`source must not exceed ${MAX_TYPST_SOURCE_BYTES} UTF-8 bytes`);
	}
	return { source };
}
