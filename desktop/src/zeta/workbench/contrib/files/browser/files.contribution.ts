import {
  IFileService,
} from "../../../../platform/files/common/files.js";
import {
  SyncDescriptor,
} from "../../../../platform/instantiation/common/instantiation.js";
import {
  IWorkspaceContextService,
} from "../../../../platform/workspace/common/workspace.js";
import {
  WorkbenchStateContext,
} from "../../../common/contextkeys.js";
import {
  type WorkbenchViewRegistry,
  WorkbenchViewContainerId,
  ViewsRegistry,
} from "../../../common/views.js";
import { ExplorerViewPane } from "./explorerViewPane.js";
import "./media/explorer.css";

export const EXPLORER_VIEW_ID = "zeta.explorer";

/** Registers the file views after the core Workbench containers exist. */
export function registerFilesViews(
  registry: WorkbenchViewRegistry = ViewsRegistry,
): void {
  registry.registerStaticViews(WorkbenchViewContainerId.Sidebar, [{
    id: EXPLORER_VIEW_ID,
    title: "Explorer",
    order: 1,
    when: WorkbenchStateContext.isEqualTo("folder"),
    canToggleVisibility: false,
    ctorDescriptor: new SyncDescriptor(ExplorerViewPane, {
      serviceDependencies: [
        IFileService,
        IWorkspaceContextService,
      ],
    }),
  }]);
}
