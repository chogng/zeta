import { lxiconsLibrary } from "../../../../base/common/lxiconsLibrary.js";
import { IMenuService } from "../../../../platform/actions/common/menuService.js";
import { IContextKeyService } from "../../../../platform/contextkey/common/contextkey.js";
import { IContextMenuService } from "../../../../platform/contextview/browser/contextMenu.js";
import { IHoverService } from "../../../../platform/hover/common/hoverService.js";
import { IFileIconThemeService } from "../../../../platform/theme/browser/fileIconThemeService.js";
import { SyncDescriptor } from "../../../../platform/instantiation/common/instantiation.js";
import { registerWorkbenchContribution, WorkbenchPhase } from "../../../common/contributions.js";
import { ViewContainerLocation, type WorkbenchViewRegistry, WorkbenchViewContainerId, ViewsRegistry } from "../../../common/views.js";
import { IGitService } from "../../../services/git/common/gitService.js";
import { IEditorService } from "../../../services/editor/common/editorService.js";
import { IStatusbarService } from "../../../services/statusbar/browser/statusbar.js";
import { IViewsService } from "../../../services/views/browser/viewsService.js";
import { ScmAgentReviewViewPane } from "./scmAgentReviewViewPane.js";
import { ScmGraphViewPane } from "./scmGraphViewPane.js";
import { ScmStatusContribution } from "./scmStatus.js";
import { ScmViewPane } from "./scmViewPane.js";
import "./media/scm.css";
import { isRemoteResource } from "../../../../platform/remote/common/remote.js";
import { registerEditorDecorationSourceFactory } from "../../../browser/parts/editor/editorDecorations.js";
import { DirtyDiffDecorationSource } from "./dirtyDiffDecorationSource.js";

export const GIT_VIEW_ID = "zeta.gitView";
export const GIT_AGENT_REVIEW_VIEW_ID = "zeta.gitAgentReview";
export const GIT_GRAPH_VIEW_ID = "zeta.gitGraph";

registerEditorDecorationSourceFactory(({ accessor, diffApi, model, resource }) => {
  if (!diffApi || (resource.scheme !== "file" && !isRemoteResource(resource))) return undefined;
  return new DirtyDiffDecorationSource(resource, model, accessor.get(IGitService), diffApi);
});

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
        serviceDependencies: [IGitService, IFileIconThemeService, IEditorService],
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
        serviceDependencies: [IGitService, IMenuService, IContextMenuService, IContextKeyService, IHoverService, IEditorService, IFileIconThemeService],
      }),
    },
  ]);
}

registerWorkbenchContribution("workbench.contrib.scmStatus", WorkbenchPhase.BlockRestore, accessor => new ScmStatusContribution({
  statusbarService: accessor.get(IStatusbarService),
  gitService: accessor.get(IGitService),
  viewsService: accessor.get(IViewsService),
}));
