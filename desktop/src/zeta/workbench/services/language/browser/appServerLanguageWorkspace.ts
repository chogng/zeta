import { type IWorkspaceContextService } from "../../../../platform/workspace/common/workspace.js";
import { type IWorkspaceTrustService } from "../../../../platform/workspaceTrust/common/workspaceTrustService.js";

export interface AppServerLanguageWorkspaceTrust {
  readonly workspaceId: string;
  readonly trusted: boolean;
}

/** Resolves whether the current single-folder Workspace may activate executable language services. */
export async function resolveAppServerLanguageWorkspaceTrust(workspaceContext: IWorkspaceContextService, workspaceTrust?: IWorkspaceTrustService): Promise<AppServerLanguageWorkspaceTrust> {
  const workspace = workspaceContext.getWorkspace();
  if (workspace.folders.length !== 1) return { workspaceId: workspace.id, trusted: false };
  if (!workspaceTrust) return { workspaceId: workspace.id, trusted: true };
  const snapshot = await workspaceTrust.list();
  return { workspaceId: workspace.id, trusted: snapshot.entries.some(entry => entry.workspace === workspace.id && entry.setting === "trusted") };
}
