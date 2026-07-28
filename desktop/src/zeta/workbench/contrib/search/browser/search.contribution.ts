import { LxIcon } from "../../../../base/common/lxicons.js";
import {
  SyncDescriptor,
} from "../../../../platform/instantiation/common/instantiation.js";
import {
  PlaceholderViewPane,
} from "../../../browser/parts/views/placeholderViewPane.js";
import {
  ViewContainerLocation,
  type WorkbenchViewRegistry,
  WorkbenchViewContainerId,
  ViewsRegistry,
} from "../../../common/views.js";

export const SEARCH_VIEW_ID = "zeta.searchView";

/** Registers the Search Sidebar container and its initial pane. */
export function registerSearchViews(
  registry: WorkbenchViewRegistry = ViewsRegistry,
): void {
  registry.registerStaticViewContainer({
    id: WorkbenchViewContainerId.Search,
    title: "Search",
    location: ViewContainerLocation.Sidebar,
    icon: LxIcon.search,
    order: 2,
  });
  registry.registerStaticViews(WorkbenchViewContainerId.Search, [{
    id: SEARCH_VIEW_ID,
    title: "Search",
    order: 1,
    canToggleVisibility: false,
    ctorDescriptor: new SyncDescriptor(PlaceholderViewPane, {
      staticArguments: ["Workspace search is not implemented yet."],
    }),
  }]);
}
