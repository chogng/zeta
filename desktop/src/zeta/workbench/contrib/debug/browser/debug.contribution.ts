import { lxiconsLibrary } from "../../../../base/common/lxiconsLibrary.js";
import { SyncDescriptor } from "../../../../platform/instantiation/common/instantiation.js";
import { ViewContainerLocation, type WorkbenchViewRegistry, WorkbenchViewContainerId, ViewsRegistry } from "../../../common/views.js";
import { IEditorPart } from "../../../browser/parts/editor/editorPart.js";
import { IDebugService } from "../../../services/debug/common/debugService.js";
import { DEBUG_VIEW_ID } from "../common/debug.js";
import { DebugViewPane } from "./debugViewPane.js";
import { registerEditorLineGutterDecorationFactory } from "../../../browser/parts/editor/editorGutterDecorations.js";
import { DebugBreakpointDecorationProvider } from "./debugBreakpointDecorations.js";
import { registerWorkbenchServiceContribution } from "../../../browser/workbenchServiceContributions.js";
import { DebugService } from "../../../services/debug/browser/debugService.js";
import { ITaskService } from "../../../services/tasks/common/taskService.js";
import { isRemoteResource } from "../../../../platform/remote/common/remote.js";
import "./debugActions.js";
import "./media/debug.css";

export function registerDebugView(registry: WorkbenchViewRegistry = ViewsRegistry): void {
  registry.registerStaticViewContainer({ id: WorkbenchViewContainerId.Debug, title: "Run and Debug", location: ViewContainerLocation.Sidebar, icon: lxiconsLibrary.start, order: 3 });
  registry.registerStaticViews(WorkbenchViewContainerId.Debug, [{ id: DEBUG_VIEW_ID, title: "Run and Debug", order: 1, canToggleVisibility: false, ctorDescriptor: new SyncDescriptor(DebugViewPane, { serviceDependencies: [IDebugService, IEditorPart] }) }]);
}

registerDebugView();
registerWorkbenchServiceContribution(context => context.services.set(IDebugService, context.own(new DebugService(context.fileService, context.workspaceContext, context.rendererHost.debugAdapter, context.terminalService, context.storageService, context.services.get(ITaskService)))));
registerEditorLineGutterDecorationFactory((resource, accessor) => resource.scheme === "file" || isRemoteResource(resource) ? new DebugBreakpointDecorationProvider(accessor.get(IDebugService), resource) : undefined);
