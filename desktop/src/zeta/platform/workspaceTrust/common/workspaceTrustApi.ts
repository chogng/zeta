import type { ConfigCommandResult, WorkspaceTrustForgetParams, WorkspaceTrustListResult, WorkspaceTrustSetParams } from "../../../../../generated/app-server/types.js";

/** Transport-only Workspace Trust management operations. */
export interface IWorkspaceTrustApi {
  list(): Promise<WorkspaceTrustListResult>;
  set(params: WorkspaceTrustSetParams): Promise<ConfigCommandResult>;
  forget(params: WorkspaceTrustForgetParams): Promise<ConfigCommandResult>;
}
