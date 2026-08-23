import { APP_SERVER_METHODS, type SkillListParams } from "../../../../../generated/app-server/types.js";
import type { AppServerSupervisor } from "../../app-server/electron-main/app-server-supervisor.js";
import { record, stringEnum } from "../../ipc/electron-main/ipcValidation.js";
import type { IpcRoute } from "../../ipc/electron-main/trustedIpcRouter.js";

/** Exact-shape IPC routes for the App Server-owned metadata-only Skill catalog. */
export function skillIpcRoutes(supervisor: AppServerSupervisor): readonly IpcRoute<unknown, unknown>[] {
	return [route({
		channel: "zeta:skills:list",
		validate: skillListParams,
		invoke: params => supervisor.request(APP_SERVER_METHODS["skills/list"], params),
	})];
}

function route<P, R>(definition: IpcRoute<P, R>): IpcRoute<unknown, unknown> {
	return { channel: definition.channel, validate: definition.validate, invoke: params => definition.invoke(params as P) };
}

function skillListParams(value: unknown): SkillListParams {
	const params = record(value, ["reload"]);
	return { reload: stringEnum(params.reload, "reload", ["cached", "refresh"] as const) };
}
