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
  const folder = workspace.folders[0];
  if (folder.uri.scheme !== "file") return { workspaceId: workspace.id, trusted: false };
  const state = await workspaceTrust.read(folder.uri.fsPath);
  return { workspaceId: workspace.id, trusted: state === "trusted" };
}
