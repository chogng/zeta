import { Emitter } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import type {
	IAnyWorkspaceIdentifier,
	IWorkspace,
	IWorkspaceChangeEvent,
	IWorkspaceContextService,
	IWorkspaceFolder,
} from "../../../../platform/workspace/common/workspace.js";
import {
	WorkbenchState,
	workspaceFromIdentifier,
} from "../../../../platform/workspace/common/workspace.js";

/** Live renderer projection of the workspace hosted by this window. */
export class WorkspaceContextService extends DisposableOwner implements IWorkspaceContextService {
	private readonly _onDidChangeWorkspace =
		this.own(new Emitter<IWorkspaceChangeEvent>());
	private workspace: IWorkspace;

	readonly onDidChangeWorkspace = this._onDidChangeWorkspace.event;

	constructor(workspace: IAnyWorkspaceIdentifier | IWorkspace) {
		super();
		this.workspace = resolveWorkspace(workspace);
	}

	getWorkspace(): IWorkspace {
		return this.workspace;
	}

	getWorkbenchState(): WorkbenchState {
		if (this.workspace.configuration) {
			return WorkbenchState.WORKSPACE;
		}
		if (this.workspace.folders.length === 1) {
			return WorkbenchState.FOLDER;
		}
		return WorkbenchState.EMPTY;
	}

	/** Atomically replaces the current window workspace and publishes its projection. */
	updateWorkspace(next: IAnyWorkspaceIdentifier | IWorkspace): void {
		const workspace = resolveWorkspace(next);
		const previous = this.workspace;
		if (previous.id === workspace.id) return;
		this.workspace = workspace;
		this._onDidChangeWorkspace.fire({ previous, workspace });
	}
}

function resolveWorkspace(
	workspace: IAnyWorkspaceIdentifier | IWorkspace,
): IWorkspace {
	if (!('folders' in workspace)) return workspaceFromIdentifier(workspace);
	const folders: readonly IWorkspaceFolder[] = Object.freeze(workspace.folders.map(folder => Object.freeze({ ...folder })));
	return Object.freeze({ ...workspace, folders });
}
