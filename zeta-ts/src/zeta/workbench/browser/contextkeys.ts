import { type IDisposable, toDisposable } from '../../base/common/lifecycle.js';
import type { IContextKeyService } from '../../platform/contextkey/common/contextkey.js';
import { type IWorkspaceContextService, workbenchStateToString } from '../../platform/workspace/common/workspace.js';
import { DirtyWorkingCopiesContext, WorkbenchStateContext, WorkspaceFolderCountContext } from '../common/contextkeys.js';
import type { IWorkingCopyService } from '../services/workingCopy/common/workingCopyService.js';

/** Keeps window-wide Workbench context keys synchronized with the workspace. */
export function bindWorkbenchContextKeys(
	contextKeyService: IContextKeyService,
	workspaceContextService: IWorkspaceContextService,
	workingCopyService: IWorkingCopyService,
): IDisposable {
	const { workbenchState, workspaceFolderCount, dirtyWorkingCopies } = bufferContextKeyChanges(contextKeyService, () => {
		const workbenchState = WorkbenchStateContext.bindTo(contextKeyService);
		const workspaceFolderCount = WorkspaceFolderCountContext.bindTo(contextKeyService);
		const dirtyWorkingCopies = DirtyWorkingCopiesContext.bindTo(contextKeyService);
		workbenchState.set(workbenchStateToString(workspaceContextService.getWorkbenchState()));
		workspaceFolderCount.set(workspaceContextService.getWorkspace().folders.length);
		dirtyWorkingCopies.set(workingCopyService.hasDirtyWorkingCopies);
		return { workbenchState, workspaceFolderCount, dirtyWorkingCopies };
	});

	const updateWorkspaceKeys = (): void => {
		workbenchState.set(workbenchStateToString(
			workspaceContextService.getWorkbenchState(),
		));
		workspaceFolderCount.set(
			workspaceContextService.getWorkspace().folders.length,
		);
	};
	const updateDirtyWorkingCopies = (): void => {
		dirtyWorkingCopies.set(workingCopyService.hasDirtyWorkingCopies);
	};
	const workspaceSubscription =
		workspaceContextService.onDidChangeWorkspace(updateWorkspaceKeys);
	const dirtySubscription = workingCopyService.onDidChangeDirty(updateDirtyWorkingCopies);

	return toDisposable(() => {
		dirtySubscription.dispose();
		workspaceSubscription.dispose();
		contextKeyService.bufferChangeEvents(() => {
			dirtyWorkingCopies.reset();
			workspaceFolderCount.reset();
			workbenchState.reset();
		});
	});
}

function bufferContextKeyChanges<T>(contextKeyService: IContextKeyService, callback: () => T): T {
	let result!: T;
	contextKeyService.bufferChangeEvents(() => {
		result = callback();
	});
	return result;
}
