import { lxiconsLibrary } from "../../../../base/common/lxiconsLibrary.js";
import { SyncDescriptor } from "../../../../platform/instantiation/common/instantiation.js";
import { ViewContainerLocation, type WorkbenchViewRegistry, WorkbenchViewContainerId, ViewsRegistry } from "../../../common/views.js";
import { IGitService } from "../../../services/git/common/gitService.js";
import { ScmAgentReviewViewPane } from "./scmAgentReviewViewPane.js";
import { ScmGraphViewPane } from "./scmGraphViewPane.js";
import { ScmViewPane } from "./scmViewPane.js";
import "./media/scm.css";

export const GIT_VIEW_ID = "zeta.gitView";
export const GIT_AGENT_REVIEW_VIEW_ID = "zeta.gitAgentReview";
export const GIT_GRAPH_VIEW_ID = "zeta.gitGraph";

/** Registers the Git Sidebar container and its initial pane. */
export function registerGitViews(
  registry: WorkbenchViewRegistry = ViewsRegistry,
): void {
  registry.registerStaticViewContainer({
    id: WorkbenchViewContainerId.Git,
    title: "Git",
    location: ViewContainerLocation.Sidebar,
    icon: lxiconsLibrary.gitBranch,
    order: 3,
  });
  registry.registerStaticViews(WorkbenchViewContainerId.Git, [
    {
      id: GIT_VIEW_ID,
      title: "Changes",
      order: 1,
      canToggleVisibility: false,
      ctorDescriptor: new SyncDescriptor(ScmViewPane, {
        serviceDependencies: [IGitService],
      }),
    },
    {
      id: GIT_AGENT_REVIEW_VIEW_ID,
      title: "Agent Review",
      order: 2,
      collapsed: true,
      canToggleVisibility: false,
      ctorDescriptor: new SyncDescriptor(ScmAgentReviewViewPane),
    },
    {
      id: GIT_GRAPH_VIEW_ID,
      title: "Graph",
      order: 3,
      collapsed: true,
      canToggleVisibility: false,
      ctorDescriptor: new SyncDescriptor(ScmGraphViewPane, {
        serviceDependencies: [IGitService],
      }),
    },
  ]);
}
