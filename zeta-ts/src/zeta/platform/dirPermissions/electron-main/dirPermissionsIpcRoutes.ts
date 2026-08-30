import { APP_SERVER_METHODS, type PermissionDto, type DirPermissionsForgetParams, type DirPermissionsReadParams, type DirPermissionsSetParams } from "../../../../../generated/app-server/types.js";
import type { AppServerSupervisor } from "../../app-server/electron-main/app-server-supervisor.js";
import { nonEmptyString, nonNegativeInteger, record, stringEnum } from "../../ipc/electron-main/ipcValidation.js";
import type { IpcRoute } from "../../ipc/electron-main/trustedIpcRouter.js";

const CAPABILITIES = [
	"readFiles", "writeFiles", "executeCommands", "watchFiles", "browseFiles", "searchFiles",
	"loadInstructions", "loadConfig", "discoverSkills", "discoverMcp", "useLanguageServices",
	"discoverHooks", "discoverPlugins", "inspectRepository", "mutateRepository",
] as const satisfies readonly PermissionDto[];

export function dirPermissionsIpcRoutes(supervisor: AppServerSupervisor): readonly IpcRoute<unknown, unknown>[] {
	return [
		route({ channel: "zeta:dir-permissions:list", validate: emptyParams, invoke: () => supervisor.request(APP_SERVER_METHODS["config/dirPermissions/list"], {}) }),
		route({ channel: "zeta:dir-permissions:read", validate: readParams, invoke: params => supervisor.request(APP_SERVER_METHODS["config/dirPermissions/read"], params) }),
		route({ channel: "zeta:dir-permissions:set", validate: setParams, invoke: params => supervisor.request(APP_SERVER_METHODS["config/dirPermissions/set"], params) }),
		route({ channel: "zeta:dir-permissions:forget", validate: forgetParams, invoke: params => supervisor.request(APP_SERVER_METHODS["config/dirPermissions/forget"], params) }),
	];
}

function route<P, R>(definition: IpcRoute<P, R>): IpcRoute<unknown, unknown> {
	return { channel: definition.channel, validate: definition.validate, invoke: params => definition.invoke(params as P) };
}

function emptyParams(value: unknown): Record<string, never> {
	if (value === undefined) return {};
	return record(value, []) as Record<string, never>;
}

function setParams(value: unknown): DirPermissionsSetParams {
	const params = record(value, ["commandId", "expectedRevision", "path", "permissions"]);
	if (!Array.isArray(params.permissions)) throw new Error("permissions must be an array");
	return {
		commandId: nonEmptyString(params.commandId, "commandId"),
		expectedRevision: nonNegativeInteger(params.expectedRevision, "expectedRevision"),
		path: nonEmptyString(params.path, "path"),
		permissions: params.permissions.map((permission, index) => stringEnum(permission, `permissions[${index}]`, CAPABILITIES)),
	};
}

function readParams(value: unknown): DirPermissionsReadParams {
	const params = record(value, ["path"]);
	return { path: nonEmptyString(params.path, "path") };
}

function forgetParams(value: unknown): DirPermissionsForgetParams {
	const params = record(value, ["commandId", "expectedRevision", "dir"]);
	return {
		commandId: nonEmptyString(params.commandId, "commandId"),
		expectedRevision: nonNegativeInteger(params.expectedRevision, "expectedRevision"),
		dir: nonEmptyString(params.dir, "dir"),
	};
}
