import type { WorkspaceSearchReadResult, WorkspaceSearchStartResult } from "../../../../../generated/app-server/types.js";
import { invoke } from "../../ipc/electron-browser/rendererIpc.js";
import type { IWorkspaceSearchApi } from "../common/searchApi.js";

export function createWorkspaceSearchApi(): IWorkspaceSearchApi {
  return {
    start: (params) => invoke<WorkspaceSearchStartResult>("zeta:workspace-search:start", params),
    read: (params) => invoke<WorkspaceSearchReadResult>("zeta:workspace-search:read", params),
    cancel: (params) => invoke<void>("zeta:workspace-search:cancel", params),
  };
}
