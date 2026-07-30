import { lxiconsLibrary } from "../../../../base/common/lxiconsLibrary.js";
import { SyncDescriptor } from "../../../../platform/instantiation/common/instantiation.js";
import { ViewContainerLocation, type WorkbenchViewRegistry, WorkbenchViewContainerId, ViewsRegistry } from "../../../common/views.js";
import { IRendererApiService } from "../../../common/services.js";
import { ScmViewPane } from "./scmViewPane.js";
import "./media/scm.css";

export const GIT_VIEW_ID = "zeta.gitView";

/** Registers the Git Sidebar container and its initial pane. */
export function registerGitViews(
  registry: WorkbenchViewRegistry = ViewsRegistry,
): void {
  registry.registerStaticViewContainer({
    id: WorkbenchViewContainerId.Git,
    title: "Git",
    location: ViewContainerLocation.Sidebar,
    icon: lxiconsLibrary.gitCommit,
    order: 3,
  });
  registry.registerStaticViews(WorkbenchViewContainerId.Git, [{
    id: GIT_VIEW_ID,
    title: "Changes",
    order: 1,
    canToggleVisibility: false,
    ctorDescriptor: new SyncDescriptor(ScmViewPane, {
      serviceDependencies: [IRendererApiService],
    }),
  }]);
}
