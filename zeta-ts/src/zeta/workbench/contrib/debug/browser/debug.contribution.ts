import { lxiconsLibrary } from "../../../../base/common/lxiconsLibrary.js";
import { SyncDescriptor } from "../../../../platform/instantiation/common/instantiation.js";
import { ViewContainerLocation, type WorkbenchViewRegistry, WorkbenchViewContainerId, ViewsRegistry } from "../../../common/views.js";
import { IEditorService } from "../../../services/editor/common/editorService.js";
import { IDebugService } from "../../../services/debug/common/debugService.js";
import { IDebugConsoleService } from "../../../services/debug/common/debugConsoleService.js";
import { DEBUG_CONSOLE_VIEW_ID, DEBUG_VIEW_ID } from "../common/debug.js";
import { DebugViewPane } from "./debugViewPane.js";
import { DebugConsoleViewPane } from "./debugConsoleViewPane.js";
import { registerEditorLineGutterDecorationFactory } from "../../../browser/parts/editor/editorGutterDecorations.js";
import { DebugBreakpointDecorationProvider } from "./debugBreakpointDecorations.js";
import { isRemoteResource } from "../../../../platform/remote/common/remote.js";
import "./debugActions.js";
import "./media/debug.css";

export function registerDebugView(registry: WorkbenchViewRegistry = ViewsRegistry): void {
	registry.registerStaticViewContainer({ id: WorkbenchViewContainerId.Debug, title: "Run and Debug", localizationKey: { bundle: "zeta.views", key: "runAndDebug" }, location: ViewContainerLocation.Sidebar, icon: lxiconsLibrary.start, order: 3 });
	registry.registerStaticViews(WorkbenchViewContainerId.Debug, [{ id: DEBUG_VIEW_ID, title: "Run and Debug", localizationKey: { bundle: "zeta.views", key: "runAndDebug" }, order: 1, canToggleVisibility: false, ctorDescriptor: new SyncDescriptor(DebugViewPane, { serviceDependencies: [IDebugService, IEditorService] }) }]);
	registry.registerStaticViewContainer({ id: WorkbenchViewContainerId.DebugConsole, title: "Debug Console", localizationKey: { bundle: "zeta.views", key: "debugConsole" }, location: ViewContainerLocation.Panel, order: 2.75 });
	registry.registerStaticViews(WorkbenchViewContainerId.DebugConsole, [{ id: DEBUG_CONSOLE_VIEW_ID, title: "Debug Console", localizationKey: { bundle: "zeta.views", key: "debugConsole" }, order: 1, canToggleVisibility: false, ctorDescriptor: new SyncDescriptor(DebugConsoleViewPane, { serviceDependencies: [IDebugConsoleService] }) }]);
}

registerDebugView();
registerEditorLineGutterDecorationFactory((resource, accessor) => resource.scheme === "file" || isRemoteResource(resource) ? new DebugBreakpointDecorationProvider(accessor.get(IDebugService), resource) : undefined);
