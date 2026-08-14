import { SyncDescriptor } from "../../../../platform/instantiation/common/instantiation.js";
import { ViewContainerLocation, type WorkbenchViewRegistry, WorkbenchViewContainerId, ViewsRegistry } from "../../../common/views.js";
import { ILanguageServerStatusService } from "../../../services/language/common/languageServerStatusService.js";
import { LanguageServerOutputViewPane } from "../../languageServer/browser/languageServerOutputViewPane.js";

/** Registers the fixed Workbench-owned Panel destinations. */
export function registerPanelViews(registry: WorkbenchViewRegistry = ViewsRegistry): void {
  registry.registerStaticViewContainer({ id: WorkbenchViewContainerId.Output, title: "Output", location: ViewContainerLocation.Panel, order: 2 });
  registry.registerStaticViews(WorkbenchViewContainerId.Output, [{ id: "zeta.output", title: "Language Servers", order: 1, canToggleVisibility: false, ctorDescriptor: new SyncDescriptor(LanguageServerOutputViewPane, { serviceDependencies: [ILanguageServerStatusService] }) }]);
}
