import type { WorkspaceSearchCancelParams, WorkspaceSearchReadParams, WorkspaceSearchReadResult, WorkspaceSearchStartParams, WorkspaceSearchStartResult } from "../../../../../generated/app-server/types.js";

export interface IWorkspaceSearchApi {
  start(params: WorkspaceSearchStartParams): Promise<WorkspaceSearchStartResult>;
  read(params: WorkspaceSearchReadParams): Promise<WorkspaceSearchReadResult>;
  cancel(params: WorkspaceSearchCancelParams): Promise<void>;
}
