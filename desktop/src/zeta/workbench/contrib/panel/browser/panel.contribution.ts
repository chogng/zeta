import { SyncDescriptor } from "../../../../platform/instantiation/common/instantiation.js";
import { PlaceholderViewPane } from "../../../browser/parts/views/placeholderViewPane.js";
import { ViewContainerLocation, type WorkbenchViewRegistry, WorkbenchViewContainerId, ViewsRegistry } from "../../../common/views.js";

const placeholderPanels = [
  {
    containerId: WorkbenchViewContainerId.Problems,
    title: "Problems",
    order: 1,
    viewId: "zeta.problems",
    message: "No problems have been detected.",
  },
  {
    containerId: WorkbenchViewContainerId.Output,
    title: "Output",
    order: 2,
    viewId: "zeta.output",
    message: "No output is available.",
  },
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
