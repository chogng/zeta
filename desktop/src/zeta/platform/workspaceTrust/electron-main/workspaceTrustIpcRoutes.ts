import { APP_SERVER_METHODS, type WorkspaceTrustForgetParams, type WorkspaceTrustSetParams } from "../../../../../generated/app-server/types.js";
import type { AppServerSupervisor } from "../../app-server/electron-main/app-server-supervisor.js";
import { nonEmptyString, nonNegativeInteger, record } from "../../ipc/electron-main/ipcValidation.js";
import type { IpcRoute } from "../../ipc/electron-main/trustedIpcRouter.js";

export function workspaceTrustIpcRoutes(supervisor: AppServerSupervisor): readonly IpcRoute<unknown, unknown>[] {
  return [
    route({ channel: "zeta:workspace-trust:list", validate: emptyParams, invoke: () => supervisor.request(APP_SERVER_METHODS["workspace/trust/list"], {}) }),
    route({ channel: "zeta:workspace-trust:set", validate: setParams, invoke: params => supervisor.request(APP_SERVER_METHODS["workspace/trust/set"], params) }),
    route({ channel: "zeta:workspace-trust:forget", validate: forgetParams, invoke: params => supervisor.request(APP_SERVER_METHODS["workspace/trust/forget"], params) }),
  ];
}

function route<P, R>(definition: IpcRoute<P, R>): IpcRoute<unknown, unknown> {
  return { channel: definition.channel, validate: definition.validate, invoke: params => definition.invoke(params as P) };
}

function emptyParams(value: unknown): Record<string, never> {
  if (value === undefined) return {};
  return record(value, []) as Record<string, never>;
}

function setParams(value: unknown): WorkspaceTrustSetParams {
  const params = record(value, ["commandId", "expectedRevision", "root", "setting"]);
  if (params.setting !== "restricted" && params.setting !== "trusted") throw new Error("setting must be restricted or trusted");
  return {
    commandId: nonEmptyString(params.commandId, "commandId"),
    expectedRevision: nonNegativeInteger(params.expectedRevision, "expectedRevision"),
    root: nonEmptyString(params.root, "root"),
    setting: params.setting,
  };
}

function forgetParams(value: unknown): WorkspaceTrustForgetParams {
  const params = record(value, ["commandId", "expectedRevision", "workspace"]);
  return {
    commandId: nonEmptyString(params.commandId, "commandId"),
    expectedRevision: nonNegativeInteger(params.expectedRevision, "expectedRevision"),
    workspace: nonEmptyString(params.workspace, "workspace"),
  };
}
