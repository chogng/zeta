import {
  type IDisposable,
  toDisposable,
} from "../../base/common/lifecycle.js";
import type {
  IContextKeyService,
} from "../../platform/contextkey/common/contextkey.js";
import type {
  IWorkspaceContextService,
} from "../../platform/workspace/common/workspace.js";
import {
  AuxiliaryBarVisibleContext,
  EditorAreaVisibleContext,
  FocusedViewContext,
  PanelVisibleContext,
  SideBarVisibleContext,
  WorkbenchStateContext,
  WorkspaceFolderCountContext,
  workbenchStateToContextValue,
} from "../common/contextkeys.js";
import type {
  IWorkbenchLayoutService,
  WorkbenchPartId,
} from "./layout.js";

/** Installs the initial window-wide Workbench context keys. */
export function bindWorkbenchContextKeys(
  contextKeyService: IContextKeyService,
  workspaceContextService: IWorkspaceContextService,
): IDisposable {
  const workbenchState = WorkbenchStateContext.bindTo(contextKeyService);
  const workspaceFolderCount =
    WorkspaceFolderCountContext.bindTo(contextKeyService);
  const sideBarVisible = SideBarVisibleContext.bindTo(contextKeyService);
  const auxiliaryBarVisible =
    AuxiliaryBarVisibleContext.bindTo(contextKeyService);
  const panelVisible = PanelVisibleContext.bindTo(contextKeyService);
  const editorAreaVisible =
    EditorAreaVisibleContext.bindTo(contextKeyService);
  const focusedView = FocusedViewContext.bindTo(contextKeyService);

  workbenchState.set(workbenchStateToContextValue(
    workspaceContextService.getWorkbenchState(),
  ));
  workspaceFolderCount.set(
    workspaceContextService.getWorkspace().folders.length,
  );

  return toDisposable(() => {
    focusedView.reset();
    editorAreaVisible.reset();
    panelVisible.reset();
    auxiliaryBarVisible.reset();
    sideBarVisible.reset();
    workspaceFolderCount.reset();
    workbenchState.reset();
  });
}

/** Keeps layout-owned visibility keys synchronized with the browser shell. */
export function bindWorkbenchPartVisibilityContextKeys(
  contextKeyService: IContextKeyService,
  layoutService: IWorkbenchLayoutService,
): IDisposable {
  const update = (
    partId: WorkbenchPartId,
    visible: boolean,
  ) => applyWorkbenchPartVisibilityContext(
    contextKeyService,
    partId,
    visible,
  );
  for (const partId of visibilityContextPartIds) {
    update(partId, layoutService.isPartVisible(partId));
  }
  return layoutService.onDidChangePartVisibility(({ partId, visible }) => {
    update(partId, visible);
  });
}

const visibilityContextPartIds = [
  "sidebar",
  "auxiliarybar",
  "panel",
  "editor",
] as const satisfies readonly WorkbenchPartId[];

function applyWorkbenchPartVisibilityContext(
  contextKeyService: IContextKeyService,
  partId: WorkbenchPartId,
  visible: boolean,
): void {
  switch (partId) {
    case "sidebar":
      contextKeyService.setContext(SideBarVisibleContext.key, visible);
      break;
    case "auxiliarybar":
      contextKeyService.setContext(AuxiliaryBarVisibleContext.key, visible);
      break;
    case "panel":
      contextKeyService.setContext(PanelVisibleContext.key, visible);
      break;
    case "editor":
      contextKeyService.setContext(EditorAreaVisibleContext.key, visible);
      break;
  }
}
