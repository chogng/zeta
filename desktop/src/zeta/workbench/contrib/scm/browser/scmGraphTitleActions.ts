import { lxiconsLibrary } from "../../../../base/common/lxiconsLibrary.js";
import type { Icon } from "../../../../base/common/icon.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { MenuWorkbenchToolBar } from "../../../../platform/actions/browser/toolbar.js";
import { MenuId, MenusRegistry } from "../../../../platform/actions/common/actions.js";
import type { IMenuService } from "../../../../platform/actions/common/menuService.js";
import { CommandsRegistry } from "../../../../platform/commands/common/commands.js";
import { type IContextKey, type IContextKeyService, RawContextKey } from "../../../../platform/contextkey/common/contextkey.js";
import type { IContextMenuService } from "../../../../platform/contextview/browser/contextMenu.js";
import type { GitStatus, IGitService } from "../../../services/git/common/gitService.js";

const GitFetchCommandId = "zeta.git.fetch";
const GitPullCommandId = "zeta.git.pull";
const GitPushCommandId = "zeta.git.push";
const GitGraphRefreshCommandId = "zeta.git.graph.refresh";
const GitGraphBusyContext = new RawContextKey<boolean>("gitGraphBusy", false);

export interface ScmGraphTitleActionsOptions {
  readonly ownerDocument: Document;
  readonly gitService: IGitService;
  readonly menuService: IMenuService;
  readonly contextMenuService: IContextMenuService;
  readonly contextKeyService: IContextKeyService;
  readonly refreshGraph: () => Promise<void>;
}

/** Owns the menu-backed Fetch, Pull, Push, and Refresh actions in the Graph pane title. */
export class ScmGraphTitleActions extends DisposableOwner {
  readonly element: HTMLElement;
  private readonly toolbar: MenuWorkbenchToolBar;
  private readonly busyContext: IContextKey<boolean>;

  constructor(private readonly options: ScmGraphTitleActionsOptions) {
    super();
    this.busyContext = GitGraphBusyContext.bindTo(options.contextKeyService);
    this.defer(() => this.busyContext.reset());
    this.registerCommandsAndMenu();
    this.toolbar = this.own(new MenuWorkbenchToolBar(
      options.menuService,
      options.contextMenuService,
      MenuId.GitGraphTitle,
      options.ownerDocument,
      { ariaLabel: "Git graph actions" },
    ));
    this.element = this.toolbar.element;
    this.element.classList.add("zeta-scm-remote-actions");
  }

  private registerCommandsAndMenu(): void {
    this.own(CommandsRegistry.register(GitFetchCommandId, () => this.runRemote(() => this.options.gitService.fetch())));
    this.own(CommandsRegistry.register(GitPullCommandId, () => this.runRemote(() => this.options.gitService.pull())));
    this.own(CommandsRegistry.register(GitPushCommandId, () => this.runRemote(() => this.options.gitService.push())));
    this.own(CommandsRegistry.register(GitGraphRefreshCommandId, () => this.refreshGraph()));
    this.appendMenuItem(GitFetchCommandId, "Fetch", "Fetch Git remotes", lxiconsLibrary.repoFetch, 1);
    this.appendMenuItem(GitPullCommandId, "Pull", "Pull current branch (fast-forward only)", lxiconsLibrary.repoPull, 2);
    this.appendMenuItem(GitPushCommandId, "Push", "Push current branch", lxiconsLibrary.repoPush, 3);
    this.appendMenuItem(GitGraphRefreshCommandId, "Refresh", "Refresh Git graph", lxiconsLibrary.refresh, 4);
  }

  private appendMenuItem(id: string, title: string, tooltip: string, icon: Icon, order: number): void {
    this.own(MenusRegistry.appendMenuItem(MenuId.GitGraphTitle, {
      command: {
        id,
        title,
        tooltip,
        icon,
        precondition: GitGraphBusyContext.isEqualTo(false),
      },
      group: "navigation",
      order,
    }));
  }

  private async runRemote(operation: () => Promise<GitStatus>): Promise<void> {
    this.busyContext.set(true);
    try {
      await operation();
      await this.options.refreshGraph();
    } finally {
      this.busyContext.set(false);
    }
  }

  private async refreshGraph(): Promise<void> {
    this.busyContext.set(true);
    try {
      await this.options.refreshGraph();
    } finally {
      this.busyContext.set(false);
    }
  }
}
