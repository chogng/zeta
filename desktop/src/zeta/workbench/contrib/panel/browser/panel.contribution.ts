import { SyncDescriptor } from "../../../../platform/instantiation/common/instantiation.js";
import { PlaceholderViewPane } from "../../../browser/parts/views/placeholderViewPane.js";
import { ViewContainerLocation, type WorkbenchViewRegistry, WorkbenchViewContainerId, ViewsRegistry } from "../../../common/views.js";
import { ILanguageServerStatusService } from "../../../services/language/common/languageServerStatusService.js";
import { LanguageServerOutputViewPane } from "../../languageServer/browser/languageServerOutputViewPane.js";

const placeholderPanels = [
  {
    containerId: WorkbenchViewContainerId.Ports,
    title: "Ports",
    order: 4,
    viewId: "zeta.ports",
    message: "No forwarded ports.",
  },
] as const;

/** Registers the fixed non-terminal Panel destinations. */
export function registerPanelPlaceholderViews(registry: WorkbenchViewRegistry = ViewsRegistry): void {
  registry.registerStaticViewContainer({ id: WorkbenchViewContainerId.Output, title: "Output", location: ViewContainerLocation.Panel, order: 2 });
  registry.registerStaticViews(WorkbenchViewContainerId.Output, [{ id: "zeta.output", title: "Language Servers", order: 1, canToggleVisibility: false, ctorDescriptor: new SyncDescriptor(LanguageServerOutputViewPane, { serviceDependencies: [ILanguageServerStatusService] }) }]);
  for (const panel of placeholderPanels) {
    registry.registerStaticViewContainer({
      id: panel.containerId,
      title: panel.title,
      location: ViewContainerLocation.Panel,
      order: panel.order,
    });
    registry.registerStaticViews(panel.containerId, [{
      id: panel.viewId,
      title: panel.title,
      order: 1,
      canToggleVisibility: false,
      ctorDescriptor: new SyncDescriptor(PlaceholderViewPane, {
        staticArguments: [panel.message],
      }),
    }]);
  }
}
