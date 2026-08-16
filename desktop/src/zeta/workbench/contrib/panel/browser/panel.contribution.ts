import { SyncDescriptor } from "../../../../platform/instantiation/common/instantiation.js";
import { IContextMenuService } from "../../../../platform/contextview/browser/contextMenu.js";
import { IStorageService } from "../../../../platform/storage/common/storage.js";
import { IWorkspaceContextService } from "../../../../platform/workspace/common/workspace.js";
import { IEditorPart } from "../../../browser/parts/editor/editorPart.js";
import { IWorkbenchWindowService } from "../../../browser/window.js";
import { ViewContainerLocation, type WorkbenchViewRegistry, WorkbenchViewContainerId, ViewsRegistry } from "../../../common/views.js";
import { IOutputService } from "../../../services/output/common/outputService.js";
import { OUTPUT_VIEW_ID } from "../../output/common/output.js";
import { OutputViewPane } from "../../output/browser/outputViewPane.js";
import "../../output/browser/outputActions.js";

/** Registers the fixed Workbench-owned Panel destinations. */
export function registerPanelViews(registry: WorkbenchViewRegistry = ViewsRegistry): void {
  registry.registerStaticViewContainer({ id: WorkbenchViewContainerId.Output, title: "Output", location: ViewContainerLocation.Panel, order: 2 });
  registry.registerStaticViews(WorkbenchViewContainerId.Output, [{ id: OUTPUT_VIEW_ID, title: "Output", order: 1, canToggleVisibility: false, ctorDescriptor: new SyncDescriptor(OutputViewPane, { serviceDependencies: [IOutputService, IContextMenuService, IStorageService, IEditorPart, IWorkspaceContextService, IWorkbenchWindowService] }) }]);
}
