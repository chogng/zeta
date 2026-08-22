import type { ConfigCommandResult, WorkspaceTrustForgetParams, WorkspaceTrustListResult, WorkspaceTrustReadParams, WorkspaceTrustReadResult, WorkspaceTrustSetParams } from "../../../../../generated/app-server/types.js";
import { invoke } from "../../ipc/electron-browser/rendererIpc.js";
import type { IWorkspaceTrustApi } from "../common/workspaceTrustApi.js";

export function createWorkspaceTrustApi(): IWorkspaceTrustApi {
  return {
    list: () => invoke<WorkspaceTrustListResult>("zeta:workspace-trust:list"),
    read: params => invoke<WorkspaceTrustReadResult>("zeta:workspace-trust:read", params as WorkspaceTrustReadParams),
    set: params => invoke<ConfigCommandResult>("zeta:workspace-trust:set", params as WorkspaceTrustSetParams),
    forget: params => invoke<ConfigCommandResult>("zeta:workspace-trust:forget", params as WorkspaceTrustForgetParams),
  };
}
