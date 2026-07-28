import { toDisposable, } from "../../base/common/lifecycle.js";
import { RawContextKey, } from "../../platform/contextkey/common/contextkey.js";
/** Kind of workspace currently hosted by the window. */
export const WorkbenchStateContext = new RawContextKey("workbenchState", "empty");
/** Number of root folders in the current workspace. */
export const WorkspaceFolderCountContext = new RawContextKey("workspaceFolderCount", 0);
/** Whether the primary side bar is currently visible. */
export const SideBarVisibleContext = new RawContextKey("sideBarVisible", true);
/** Whether the auxiliary side bar is currently visible. */
export const AuxiliaryBarVisibleContext = new RawContextKey("auxiliaryBarVisible", true);
/** Whether the main editor area is currently visible. */
export const EditorAreaVisibleContext = new RawContextKey("editorAreaVisible", true);
/** Identifier of the view that currently owns keyboard focus. */
export const FocusedViewContext = new RawContextKey("focusedView", "");
/**
 * Installs the initial window-wide workbench keys.
 *
 * The workspace service is currently immutable for a window. Layout and view
 * owners can update their corresponding bound keys when those features become
 * mutable.
 */
export function bindWorkbenchContextKeys(contextKeyService, workspaceContextService) {
    const workbenchState = WorkbenchStateContext.bindTo(contextKeyService);
    const workspaceFolderCount = WorkspaceFolderCountContext.bindTo(contextKeyService);
    const sideBarVisible = SideBarVisibleContext.bindTo(contextKeyService);
    const auxiliaryBarVisible = AuxiliaryBarVisibleContext.bindTo(contextKeyService);
    const editorAreaVisible = EditorAreaVisibleContext.bindTo(contextKeyService);
    const focusedView = FocusedViewContext.bindTo(contextKeyService);
    workbenchState.set(workbenchStateToContextValue(workspaceContextService.getWorkbenchState()));
    workspaceFolderCount.set(workspaceContextService.getWorkspace().folders.length);
    return toDisposable(() => {
        focusedView.reset();
        editorAreaVisible.reset();
        auxiliaryBarVisible.reset();
        sideBarVisible.reset();
        workspaceFolderCount.reset();
        workbenchState.reset();
    });
}
/** Converts the workspace model enum into its stable context-key value. */
export function workbenchStateToContextValue(state) {
    switch (state) {
        case 1 /* WorkbenchState.EMPTY */:
            return "empty";
        case 2 /* WorkbenchState.FOLDER */:
            return "folder";
        case 3 /* WorkbenchState.WORKSPACE */:
            return "workspace";
    }
}
/** Returns the context key used to expose one view's visibility. */
export function getVisibleViewContextKey(viewId) {
    if (!viewId.trim()) {
        throw new TypeError("View ID must not be empty");
    }
    return `view.${viewId}.visible`;
}
