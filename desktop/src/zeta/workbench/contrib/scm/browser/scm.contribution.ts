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

export const GIT_VIEW_ID = "zeta.gitView";

/** Registers the Git Sidebar container and its initial pane. */
export function registerGitViews(
  registry: WorkbenchViewRegistry = ViewsRegistry,
): void {
  registry.registerStaticViewContainer({
    id: WorkbenchViewContainerId.Git,
    title: "Git",
    location: ViewContainerLocation.Sidebar,
    icon: LxIcon.gitCommit,
    order: 3,
  });
  registry.registerStaticViews(WorkbenchViewContainerId.Git, [{
    id: GIT_VIEW_ID,
    title: "Changes",
    order: 1,
    canToggleVisibility: false,
    ctorDescriptor: new SyncDescriptor(PlaceholderViewPane, {
      staticArguments: ["Git integration is not implemented yet."],
    }),
  }]);
}
