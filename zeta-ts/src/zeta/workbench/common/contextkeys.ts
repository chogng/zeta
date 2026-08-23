import {
  RawContextKey,
} from "../../platform/contextkey/common/contextkey.js";
import {
  WorkbenchState,
} from "../../platform/workspace/common/workspace.js";

/** String representation exposed to workbench context expressions. */
export type WorkbenchStateContextValue =
  | "empty"
  | "folder"
  | "workspace";

/** Kind of workspace currently hosted by the window. */
export const WorkbenchStateContext =
  new RawContextKey<WorkbenchStateContextValue>(
    "workbenchState",
    "empty",
  );

/** Number of root folders in the current workspace. */
export const WorkspaceFolderCountContext =
  new RawContextKey<number>("workspaceFolderCount", 0);

/** Whether the primary side bar is currently visible. */
export const SideBarVisibleContext =
  new RawContextKey<boolean>("sideBarVisible", true);

/** Whether the auxiliary side bar is currently visible. */
export const AuxiliaryBarVisibleContext =
  new RawContextKey<boolean>("auxiliaryBarVisible", true);

/** Whether the Workbench Agent Sidebar is currently visible. */
export const AgentSidebarVisibleContext =
  new RawContextKey<boolean>("agentSidebarVisible", false);

/** Whether the bottom panel is currently visible. */
export const PanelVisibleContext =
  new RawContextKey<boolean>("panelVisible", true);

/** Whether the main editor area is currently visible. */
export const EditorAreaVisibleContext =
  new RawContextKey<boolean>("editorAreaVisible", true);

/** Identifier of the view that currently owns keyboard focus. */
export const FocusedViewContext =
  new RawContextKey<string>("focusedView", "");

/** Converts the workspace model enum into its stable context-key value. */
export function workbenchStateToContextValue(
  state: WorkbenchState,
): WorkbenchStateContextValue {
  switch (state) {
    case WorkbenchState.EMPTY:
      return "empty";
    case WorkbenchState.FOLDER:
      return "folder";
    case WorkbenchState.WORKSPACE:
      return "workspace";
  }
}

/** Returns the context key used to expose one view's visibility. */
export function getVisibleViewContextKey(viewId: string): string {
  if (!viewId.trim()) {
    throw new TypeError("View ID must not be empty");
  }
  return `view.${viewId}.visible`;
}
