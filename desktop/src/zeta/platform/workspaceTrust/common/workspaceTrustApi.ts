import type { ConfigCommandResult, WorkspaceTrustForgetParams, WorkspaceTrustListResult, WorkspaceTrustReadParams, WorkspaceTrustReadResult, WorkspaceTrustSetParams } from "../../../../../generated/app-server/types.js";

/** Transport-only Workspace Trust management operations. */
export interface IWorkspaceTrustApi {
  list(): Promise<WorkspaceTrustListResult>;
  read(params: WorkspaceTrustReadParams): Promise<WorkspaceTrustReadResult>;
  set(params: WorkspaceTrustSetParams): Promise<ConfigCommandResult>;
  forget(params: WorkspaceTrustForgetParams): Promise<ConfigCommandResult>;
}
