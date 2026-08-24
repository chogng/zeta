import { type IWorkspaceContextService } from "../../../../platform/workspace/common/workspace.js";
import { type IWorkspaceTrustService } from "../../../../platform/workspaceTrust/common/workspaceTrustService.js";

export interface AppServerLanguageWorkspaceTrust {
	readonly workspaceId: string;
	readonly trusted: boolean;
}

/** Resolves whether every folder in the current Workspace may activate executable language services. */
export async function resolveAppServerLanguageWorkspaceTrust(workspaceContext: IWorkspaceContextService, workspaceTrust?: IWorkspaceTrustService): Promise<AppServerLanguageWorkspaceTrust> {
	const workspace = workspaceContext.getWorkspace();
	if (workspace.folders.length === 0 || workspace.folders.some(folder => folder.uri.scheme !== "file")) return { workspaceId: workspace.id, trusted: false };
	if (!workspaceTrust) return { workspaceId: workspace.id, trusted: true };
	const states = await Promise.all(workspace.folders.map(folder => workspaceTrust.read(folder.uri.fsPath)));
	return { workspaceId: workspace.id, trusted: states.every(state => state === "trusted") };
}
