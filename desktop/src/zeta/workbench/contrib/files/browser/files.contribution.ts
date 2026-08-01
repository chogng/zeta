import {
  IFileService,
} from "../../../../platform/files/common/files.js";
import {
  ContextKeyExpr,
} from "../../../../platform/contextkey/common/contextkey.js";
import {
  SyncDescriptor,
} from "../../../../platform/instantiation/common/instantiation.js";
import {
  IWorkspaceContextService,
} from "../../../../platform/workspace/common/workspace.js";
import {
  WorkspaceFolderCountContext,
} from "../../../common/contextkeys.js";
import {
  type WorkbenchViewRegistry,
  WorkbenchViewContainerId,
  ViewsRegistry,
} from "../../../common/views.js";
import {
  IWorkspaceOpenService,
} from "../../../services/workspaces/browser/workspaceOpenService.js";
import {
  IEditorPart,
} from "../../../browser/parts/editor/editorPart.js";
import {
  IFileIconThemeService,
} from "../../../../platform/theme/browser/fileIconThemeService.js";
import { IHoverService } from "../../../../platform/hover/common/hoverService.js";
import { ExplorerViewPane } from "./explorerViewPane.js";
import { EmptyView } from "./views/emptyView.js";
import "./media/explorer.css";

export const EXPLORER_VIEW_ID = "zeta.explorer";

/** Registers the file views after the core Workbench containers exist. */
export function registerFilesViews(
  registry: WorkbenchViewRegistry = ViewsRegistry,
): void {
  registry.registerStaticViews(WorkbenchViewContainerId.Sidebar, [
    {
      id: EXPLORER_VIEW_ID,
      title: "Explorer",
      order: 1,
      when: ContextKeyExpr.notEquals(WorkspaceFolderCountContext.key, 0),
      canToggleVisibility: false,
      ctorDescriptor: new SyncDescriptor(ExplorerViewPane, {
        serviceDependencies: [
          IFileService,
          IWorkspaceContextService,
          IEditorPart,
          IFileIconThemeService,
          IHoverService,
        ],
      }),
    },
    {
      id: EmptyView.ID,
      title: EmptyView.TITLE,
      order: 2,
      when: WorkspaceFolderCountContext.isEqualTo(0),
      canToggleVisibility: false,
      ctorDescriptor: new SyncDescriptor(EmptyView, {
        serviceDependencies: [IWorkspaceOpenService],
      }),
    },
  ]);
}
