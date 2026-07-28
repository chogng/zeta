import {
  type IWorkspaceContext,
  WorkbenchState,
} from "../../workspace/common/workspace.js";

export const WindowKind = {
  Empty: "empty",
  Workspace: "workspace",
} as const;

/** Identifies whether a native window hosts an empty or workspace workbench. */
export type WindowKind = typeof WindowKind[keyof typeof WindowKind];

/** Dimensions used for a new window without an opened workspace. */
export const DEFAULT_EMPTY_WINDOW_SIZE = {
  width: 1200,
  height: 800,
} as const;

/** Dimensions used for a new window with an opened workspace. */
export const DEFAULT_WORKSPACE_WINDOW_SIZE = {
  width: 1440,
  height: 900,
} as const;

/** Lower bounds that keep the workbench usable while resizing. */
export const WINDOW_MINIMUM_SIZE = {
  width: 400,
  height: 270,
} as const;

/** Maps a concrete project context to the native window policy it requires. */
export function windowKindForWorkspace(
  workspace: IWorkspaceContext,
): WindowKind {
  return workspace.state === WorkbenchState.EMPTY
    ? WindowKind.Empty
    : WindowKind.Workspace;
}
