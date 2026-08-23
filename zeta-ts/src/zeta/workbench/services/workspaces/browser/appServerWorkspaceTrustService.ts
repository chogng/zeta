import type { IWorkspaceTrustApi } from "../../../../platform/workspaceTrust/common/workspaceTrustApi.js";
import type { IWorkspaceTrustService, WorkspaceTrustCommandResult, WorkspaceTrustSetting, WorkspaceTrustSnapshot, WorkspaceTrustState } from "../../../../platform/workspaceTrust/common/workspaceTrustService.js";

/** App Server transport adapter for the Workbench Workspace Trust service contract. */
export class AppServerWorkspaceTrustService implements IWorkspaceTrustService {
	constructor(private readonly api: IWorkspaceTrustApi) {}

	async list(): Promise<WorkspaceTrustSnapshot> {
		const result = await this.api.list();
		return {
			revision: result.revision,
			entries: result.entries.map(entry => ({
				workspace: entry.workspace,
				root: entry.root ?? undefined,
			})),
		};
	}

	async read(root: string): Promise<WorkspaceTrustState> {
		return (await this.api.read({ root })).state;
	}

	async set(root: string, setting: WorkspaceTrustSetting, expectedRevision: number): Promise<WorkspaceTrustCommandResult> {
		return projectCommandResult(await this.api.set({
			commandId: commandId("set"),
			expectedRevision,
			root,
			setting,
		}));
	}

	async forget(workspace: string, expectedRevision: number): Promise<WorkspaceTrustCommandResult> {
		return projectCommandResult(await this.api.forget({
			commandId: commandId("forget"),
			expectedRevision,
			workspace,
		}));
	}
}

function projectCommandResult(result: { revision: number; generation: number; disposition: "updated" | "replayed" }): WorkspaceTrustCommandResult {
	return { revision: result.revision, generation: result.generation, disposition: result.disposition };
}

function commandId(operation: string): string {
	return `desktop-workspace-trust-${operation}-${crypto.randomUUID()}`;
}
