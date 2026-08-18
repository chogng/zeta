import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { AnchorAxisAlignment, AnchorPosition } from "../../../../../base/common/layout.js";
import type { IContextMenuService } from "../../../../../platform/contextview/browser/contextMenu.js";
import type { HoverSetupOptions, IHoverService, IManagedHover } from "../../../../../platform/hover/common/hoverService.js";
import type { IFileIconThemeService } from "../../../../../platform/theme/browser/fileIconThemeService.js";
import type { GraphQuery, GitStatus, IGitService } from "../../../../../workbench/services/git/common/gitService.js";
import type { EditorInput, EditorOpenOptions, IEditorService } from "../../../../../workbench/services/editor/common/editorService.js";

test("Git contribution registers Changes, Agent Review, and Graph as ordered panes", async () => {
  const browser = new JSDOM("<!doctype html><body></body>");
  const installedGlobals = installDomGlobals(browser);

  try {
    const { WorkbenchViewRegistry, WorkbenchViewContainerId } = await import("../../../../../workbench/common/views.js");
    const { GIT_AGENT_REVIEW_VIEW_ID, GIT_GRAPH_VIEW_ID, GIT_VIEW_ID, registerGitViews } = await import("../../../../../workbench/contrib/scm/browser/scm.contribution.js");
    const registry = new WorkbenchViewRegistry();

    registerGitViews(registry);

    const views = registry.getViews(WorkbenchViewContainerId.Git);
    assert.deepEqual(views.map((view) => view.id), [GIT_VIEW_ID, GIT_AGENT_REVIEW_VIEW_ID, GIT_GRAPH_VIEW_ID]);
    assert.deepEqual(views.map((view) => view.title), ["Changes", "Agent Review", "Graph"]);
    assert.deepEqual(views.map((view) => view.collapsed === true), [false, true, true]);
  } finally {
    browser.window.close();
    for (const name of installedGlobals) Reflect.deleteProperty(globalThis, name);
  }
});

test("ScmGraphViewPane renders a repository history page", async () => {
  const browser = new JSDOM("<!doctype html><body></body>");
  const installedGlobals = installDomGlobals(browser);
  const graphRequests: GraphQuery[] = [];
  const [
    { ContextKeyService },
    { MenuService },
    { ServiceCollection },
    { CommandService },
  ] = await Promise.all([
    import("../../../../../platform/contextkey/common/contextkey.js"),
    import("../../../../../platform/actions/common/menuService.js"),
    import("../../../../../platform/instantiation/common/instantiation.js"),
    import("../../../../../workbench/services/commands/common/commandService.js"),
  ]);
  using contextKeyService = new ContextKeyService();
  const menuService = new MenuService(new CommandService(new ServiceCollection()), contextKeyService);
  const hoverOptions: HoverSetupOptions[] = [];
  const hoverService: IHoverService = {
    setupHover: (options) => {
      hoverOptions.push(options);
      return testManagedHover();
    },
    showHover: () => testManagedHover(),
    hideHover() {},
  };
  const status: GitStatus = {
    streamInstanceId: "git-graph-stream",
    revision: 1,
    workspacePath: ".",
    head: { type: "branch", name: "main", objectId: "1234567890abcdef", upstream: { name: "origin/main", ahead: 0, behind: 0 } },
    changes: [],
  };
  const gitService = {
      status: async () => status,
      graph: async (query: GraphQuery) => {
        graphRequests.push(query);
        return {
          commits: [
            { objectId: "1234567890abcdef", parentObjectIds: ["abcdef1234567890", "side-parent"], timestampSeconds: 1_753_000_000, subject: "Wire SCM panes" },
            { objectId: "abcdef1234567890", parentObjectIds: ["parent-one", "parent-two"], timestampSeconds: 1_752_900_000, subject: "Prepare graph data" },
          ],
          references: [
            { name: "main", objectId: "1234567890abcdef", kind: "localBranch", remoteName: undefined, current: true },
            { name: "origin/main", objectId: "abcdef1234567890", kind: "remoteBranch", remoteName: "origin", current: false },
          ],
          remotes: [{ name: "origin", identity: { provider: "github", host: "github.com", owner: "chogng", repository: "zeta" } }],
          hasMore: false,
          nextCursor: undefined,
        };
      },
  } as unknown as IGitService;

  try {
    const { ScmGraphViewPane } = await import("../../../../../workbench/contrib/scm/browser/scmGraphViewPane.js");
    using pane = new ScmGraphViewPane({ id: "zeta.gitGraph.test", title: "Graph", ownerDocument: browser.window.document }, gitService, menuService, {} as IContextMenuService, contextKeyService, hoverService, testEditorService(), testFileIconThemeService());
    browser.window.document.body.append(pane.element);
    await waitFor(() => pane.element.querySelectorAll(".zeta-scm-graph-commit").length === 2);

    const remoteActionItems = [...pane.element.querySelectorAll<HTMLElement>(".zeta-pane-view-header-actions .zeta-action-view-item")];
    assert.deepEqual(remoteActionItems.map((item) => item.dataset.actionId), ["zeta.git.fetch", "zeta.git.pull", "zeta.git.push", "zeta.git.graph.refresh"]);
    assert.equal(remoteActionItems.filter((item) => item.querySelector(".zeta-icon")).length, 4);
    pane.setCollapsed(true);
    assert.equal(pane.element.querySelector<HTMLElement>(".zeta-pane-view-header-actions")?.hidden, true);
    pane.setCollapsed(false);
    assert.equal(pane.element.querySelector<HTMLElement>(".zeta-pane-view-header-actions")?.hidden, false);

    const refresh = pane.element.querySelector<HTMLButtonElement>('[data-action-id="zeta.git.graph.refresh"] > button');
    assert.ok(refresh);
    refresh.click();
    await waitFor(() => graphRequests.length === 2 && pane.element.querySelectorAll(".zeta-scm-graph-subject").length === 2);
    assert.deepEqual(graphRequests, [{ limit: 50 }, { limit: 50 }]);

    assert.deepEqual([...pane.element.querySelectorAll(".zeta-scm-graph-subject")].map((element) => element.textContent), ["Wire SCM panes", "Prepare graph data"]);
    assert.equal(pane.element.querySelector(".zeta-scm-graph-commit.current")?.getAttribute("aria-current"), "true");
    assert.ok(pane.element.querySelector(".zeta-scm-graph-commit.head"));
    assert.ok(pane.element.querySelector(".zeta-scm-graph-commit.merge"));
    assert.ok(pane.element.querySelector(".zeta-scm-graph-commit.head.merge"));
    assert.equal(pane.element.querySelector(".zeta-scm-graph-label.head")?.textContent, "main");
    assert.ok(pane.element.querySelector(".zeta-scm-graph-label.head .zeta-icon"));
    assert.equal(pane.element.querySelector(".zeta-scm-graph-label.remote")?.textContent, "origin/main");
    assert.equal(pane.element.querySelector<HTMLElement>(".zeta-scm-graph-label.remote")?.dataset.icon, "cloud");
    assert.match(pane.element.querySelector(".zeta-scm-graph-remote")?.textContent ?? "", /^GitHub · chogng\/zeta · origin$/);
    assert.equal(hoverOptions.length, 4);
    assert.ok(hoverOptions.every((options) => options.target.classList.contains("zeta-scm-graph-commit")));
    assert.ok(hoverOptions.every((options) => options.anchorAxisAlignment === AnchorAxisAlignment.Horizontal));
    assert.ok(hoverOptions.every((options) => options.anchorPosition === AnchorPosition.Below));
    assert.equal(pane.element.querySelector(".zeta-scm-graph-graph.head")?.querySelectorAll(".zeta-scm-graph-node").length, 2);
    assert.ok(new Set([...pane.element.querySelectorAll<SVGPathElement>(".zeta-scm-graph-path")].map((path) => path.dataset.laneColor)).size > 1);
    assert.equal(pane.element.querySelector(".zeta-scm-graph-commit.head.merge > .zeta-scm-graph-row > .zeta-scm-graph-graph")?.classList.contains("head"), true);
    assert.equal(pane.element.querySelector(".zeta-scm-graph-commit.head.merge > .zeta-scm-graph-row > .zeta-scm-graph-graph")?.classList.contains("merge"), false);
    assert.equal(pane.element.querySelectorAll(".zeta-scm-graph-twistie").length, 0);
    assert.equal(pane.element.querySelector(".zeta-scm-graph-graph.merge")?.querySelectorAll(".zeta-scm-graph-node").length, 2);
    assert.equal(pane.element.querySelector<SVGSVGElement>(".zeta-scm-graph-graph.merge")?.style.width, "44px");
    assert.ok((pane.element.querySelector(".zeta-scm-graph-graph.merge")?.querySelectorAll(".zeta-scm-graph-path").length ?? 0) > 1);
    assert.match(pane.element.querySelector(".zeta-scm-graph-metadata")?.textContent ?? "", /^1234567 · /);
  } finally {
    browser.window.close();
    for (const name of installedGlobals) Reflect.deleteProperty(globalThis, name);
  }
});

test("ScmGraphViewPane loads the complete history across graph pages", async () => {
  const browser = new JSDOM("<!doctype html><body></body>");
  const installedGlobals = installDomGlobals(browser);
  const graphRequests: GraphQuery[] = [];
  const [
    { ContextKeyService },
    { MenuService },
    { ServiceCollection },
    { CommandService },
  ] = await Promise.all([
    import("../../../../../platform/contextkey/common/contextkey.js"),
    import("../../../../../platform/actions/common/menuService.js"),
    import("../../../../../platform/instantiation/common/instantiation.js"),
    import("../../../../../workbench/services/commands/common/commandService.js"),
  ]);
  using contextKeyService = new ContextKeyService();
  const menuService = new MenuService(new CommandService(new ServiceCollection()), contextKeyService);
  const hoverService: IHoverService = {
    setupHover: () => testManagedHover(),
    showHover: () => testManagedHover(),
    hideHover() {},
  };
  const status: GitStatus = {
    streamInstanceId: "git-graph-stream",
    revision: 1,
    workspacePath: ".",
    head: { type: "branch", name: "main", objectId: "commit-1", upstream: undefined },
    changes: [],
  };
  const gitService = {
    status: async () => status,
    graph: async (query: GraphQuery) => {
      graphRequests.push(query);
      const firstPage = [
        { objectId: "commit-1", parentObjectIds: ["commit-2"], timestampSeconds: 1_753_000_000, subject: "First page one" },
        { objectId: "commit-2", parentObjectIds: ["commit-3"], timestampSeconds: 1_752_900_000, subject: "First page two" },
      ];
      const secondPage = [
        { objectId: "commit-3", parentObjectIds: [], timestampSeconds: 1_752_800_000, subject: "Second page one" },
      ];
      return {
        commits: query.cursor ? secondPage : firstPage,
        references: [],
        remotes: [],
        hasMore: !query.cursor,
        nextCursor: query.cursor ? undefined : "cursor-1",
      };
    },
  } as unknown as IGitService;

  try {
    const { ScmGraphViewPane } = await import("../../../../../workbench/contrib/scm/browser/scmGraphViewPane.js");
    using pane = new ScmGraphViewPane({ id: "zeta.gitGraph.pagination.test", title: "Graph", ownerDocument: browser.window.document }, gitService, menuService, {} as IContextMenuService, contextKeyService, hoverService, testEditorService(), testFileIconThemeService());
    browser.window.document.body.append(pane.element);
    await waitFor(() => pane.element.querySelector(".zeta-scm-graph-list") !== null);
    const list = pane.element.querySelector(".zeta-scm-graph-list");
    await waitFor(() => pane.element.querySelectorAll(".zeta-scm-graph-commit").length === 3);

    assert.deepEqual(graphRequests, [{ limit: 50 }, { limit: 50, cursor: "cursor-1" }]);
    assert.equal(pane.element.querySelector(".zeta-scm-graph-list"), list);
    assert.deepEqual([...pane.element.querySelectorAll(".zeta-scm-graph-subject")].map((element) => element.textContent), ["First page one", "First page two", "Second page one"]);
    assert.equal(pane.element.querySelector(".zeta-scm-graph-load-more"), null);
  } finally {
    browser.window.close();
    for (const name of installedGlobals) Reflect.deleteProperty(globalThis, name);
  }
});

test("ScmGraphViewPane virtualizes loaded history rows", async () => {
  const browser = new JSDOM("<!doctype html><body></body>");
  const installedGlobals = installDomGlobals(browser);
  let resizeCallback: ResizeObserverCallback | undefined;
  class TestResizeObserver {
    constructor(callback: ResizeObserverCallback) { resizeCallback = callback; }
    observe(): void {}
    unobserve(): void {}
    disconnect(): void {}
  }
  Object.defineProperty(browser.window, "ResizeObserver", { configurable: true, value: TestResizeObserver });
  Object.defineProperty(globalThis, "ResizeObserver", { configurable: true, value: TestResizeObserver });
  const [
    { ContextKeyService },
    { MenuService },
    { ServiceCollection },
    { CommandService },
  ] = await Promise.all([
    import("../../../../../platform/contextkey/common/contextkey.js"),
    import("../../../../../platform/actions/common/menuService.js"),
    import("../../../../../platform/instantiation/common/instantiation.js"),
    import("../../../../../workbench/services/commands/common/commandService.js"),
  ]);
  using contextKeyService = new ContextKeyService();
  const menuService = new MenuService(new CommandService(new ServiceCollection()), contextKeyService);
  const status: GitStatus = {
    streamInstanceId: "git-graph-stream",
    revision: 1,
    workspacePath: ".",
    head: { type: "branch", name: "main", objectId: "commit-0", upstream: undefined },
    changes: [],
  };
  const commits = Array.from({ length: 100 }, (_, index) => ({
    objectId: `commit-${index}`,
    parentObjectIds: index === 99 ? [] : [`commit-${index + 1}`],
    timestampSeconds: 1_753_000_000 - index,
    subject: `Commit ${index}`,
  }));
  const gitService = {
    status: async () => status,
    graph: async (_query: GraphQuery) => ({ commits, references: [], remotes: [], hasMore: false, nextCursor: undefined }),
  } as unknown as IGitService;
  const hoverService: IHoverService = {
    setupHover: () => testManagedHover(),
    showHover: () => testManagedHover(),
    hideHover() {},
  };

  try {
    const { ScmGraphViewPane } = await import("../../../../../workbench/contrib/scm/browser/scmGraphViewPane.js");
    using pane = new ScmGraphViewPane({ id: "zeta.gitGraph.virtualized.test", title: "Graph", ownerDocument: browser.window.document }, gitService, menuService, {} as IContextMenuService, contextKeyService, hoverService, testEditorService(), testFileIconThemeService());
    const graph = pane.element.querySelector<HTMLElement>(".zeta-scm-graph");
    assert.ok(graph);
    Object.defineProperty(graph, "clientHeight", { configurable: true, value: 100 });
    browser.window.document.body.append(pane.element);
    await waitFor(() => pane.element.querySelectorAll(".zeta-scm-graph-commit").length > 0);

    const initialRows = pane.element.querySelectorAll(".zeta-scm-graph-commit").length;
    assert.ok(initialRows < commits.length);
    assert.equal(pane.element.querySelectorAll(".zeta-scm-graph-spacer").length, 2);
    assert.equal(pane.element.querySelectorAll<HTMLElement>(".zeta-scm-graph-spacer")[1].style.height, `${(commits.length - initialRows) * 22}px`);

    Object.defineProperty(graph, "clientHeight", { configurable: true, value: 320 });
    resizeCallback?.([{ borderBoxSize: [{ inlineSize: 320, blockSize: 320 }], contentRect: { width: 320, height: 320 } } as unknown as ResizeObserverEntry], {} as ResizeObserver);
    assert.ok(pane.element.querySelectorAll(".zeta-scm-graph-commit").length > initialRows);

    const list = pane.element.querySelector<HTMLElement>(".zeta-scm-graph-list");
    assert.ok(list);
    Object.defineProperty(graph, "offsetParent", { configurable: true, value: browser.window.document.body });
    Object.defineProperty(list, "offsetParent", { configurable: true, value: browser.window.document.body });
    Object.defineProperty(graph, "offsetTop", { configurable: true, value: 450 });
    Object.defineProperty(list, "offsetTop", { configurable: true, value: 478 });
    Object.defineProperty(graph, "scrollTop", { configurable: true, writable: true, value: 1_000 });
    graph.dispatchEvent(new browser.window.Event("scroll"));
    assert.ok([...pane.element.querySelectorAll(".zeta-scm-graph-subject")].some((element) => element.textContent === "Commit 45"));
    assert.ok(pane.element.querySelectorAll(".zeta-scm-graph-commit").length < commits.length);
  } finally {
    Reflect.deleteProperty(globalThis, "ResizeObserver");
    browser.window.close();
    for (const name of installedGlobals) Reflect.deleteProperty(globalThis, name);
  }
});

test("ScmGraphViewPane expands commit files and opens a selected change in the diff editor", async () => {
  const browser = new JSDOM("<!doctype html><body></body>");
  const installedGlobals = installDomGlobals(browser);
  const [{ ContextKeyService }, { MenuService }, { ServiceCollection }, { CommandService }] = await Promise.all([
    import("../../../../../platform/contextkey/common/contextkey.js"),
    import("../../../../../platform/actions/common/menuService.js"),
    import("../../../../../platform/instantiation/common/instantiation.js"),
    import("../../../../../workbench/services/commands/common/commandService.js"),
  ]);
  using contextKeyService = new ContextKeyService();
  const menuService = new MenuService(new CommandService(new ServiceCollection()), contextKeyService);
  const objectId = "1".repeat(40);
  const parentObjectId = "2".repeat(40);
  let changeRequests = 0;
  let fileRequests = 0;
  const gitService = {
    status: async (): Promise<GitStatus> => ({
      streamInstanceId: "git-graph-stream",
      revision: 1,
      workspacePath: ".",
      head: { type: "branch", name: "main", objectId, upstream: undefined },
      changes: [],
    }),
    graph: async () => ({
      commits: [{ objectId, parentObjectIds: [parentObjectId], timestampSeconds: 1_753_000_000, subject: "Change editor files" }],
      references: [{ name: "main", objectId, kind: "localBranch" as const, remoteName: undefined, current: true }],
      remotes: [],
      hasMore: false,
      nextCursor: undefined,
    }),
    commitChanges: async () => {
      changeRequests += 1;
      return { parentObjectId, changes: [{ path: "src/editor.ts", originalPath: undefined, status: "modified" as const }] };
    },
    commitFile: async () => {
      fileRequests += 1;
      return { original: { kind: "text" as const, text: "before\n" }, modified: { kind: "text" as const, text: "after\n" } };
    },
  } as unknown as IGitService;
  const opened: Array<{ readonly input: EditorInput; readonly options: EditorOpenOptions | undefined }> = [];
  const editorService = testEditorService(opened);
  const hoverService: IHoverService = {
    setupHover: () => testManagedHover(),
    showHover: () => testManagedHover(),
    hideHover() {},
  };

  try {
    const { ScmGraphViewPane } = await import("../../../../../workbench/contrib/scm/browser/scmGraphViewPane.js");
    using pane = new ScmGraphViewPane({ id: "zeta.gitGraph.changes.test", title: "Graph", ownerDocument: browser.window.document }, gitService, menuService, {} as IContextMenuService, contextKeyService, hoverService, editorService, testFileIconThemeService());
    browser.window.document.body.append(pane.element);
    await waitFor(() => pane.element.querySelector(".zeta-scm-graph-commit") !== null);

    const commit = pane.element.querySelector<HTMLElement>(".zeta-scm-graph-commit");
    assert.ok(commit);
    commit.click();
    await waitFor(() => pane.element.querySelector(".zeta-scm-graph-change") !== null);
    assert.equal(changeRequests, 1);
    assert.equal(fileRequests, 0);
    assert.equal(pane.element.querySelector(".zeta-scm-graph-commit")?.getAttribute("aria-expanded"), "true");
    assert.equal(pane.element.querySelector(".zeta-scm-graph-change-label .zeta-icon-label-text")?.textContent, "editor.ts");
    assert.equal(pane.element.querySelector(".zeta-scm-graph-change-description")?.textContent, "src");
    assert.equal(pane.element.querySelector(".zeta-scm-graph-change-label .zeta-icon-label-icon")?.getAttribute("data-file-icon"), "editor.ts");
    assert.ok([...pane.element.querySelectorAll<SVGPathElement>(".zeta-scm-graph-path")].some((path) => path.getAttribute("d")?.endsWith("V 44")));
    assert.equal(commit.style.getPropertyValue("--scm-graph-node-x"), "11px");
    assert.equal(commit.style.getPropertyValue("--scm-graph-content-x"), "22px");
    assert.ok(commit.querySelector(":scope > .zeta-scm-graph-row > .zeta-scm-graph-graph"));

    const change = pane.element.querySelector<HTMLButtonElement>(".zeta-scm-graph-change");
    change?.click();
    await waitFor(() => opened.length === 1);
    assert.equal(fileRequests, 1);
    assert.equal(opened[0].input.contentType, "application/vnd.aster.editor-diff");
    assert.equal(opened[0].input.label, "editor.ts (2222222) ↔ editor.ts (1111111)");
    assert.equal(opened[0].options?.pinned, false);

    change?.dispatchEvent(new browser.window.MouseEvent("dblclick", { bubbles: true, detail: 2 }));
    await waitFor(() => opened.length === 2);
    assert.equal(fileRequests, 2);
    assert.equal(opened[1].options?.pinned, true);
  } finally {
    browser.window.close();
    for (const name of installedGlobals) Reflect.deleteProperty(globalThis, name);
  }
});

test("ScmAgentReviewViewPane exposes an explicit empty state", async () => {
  const browser = new JSDOM("<!doctype html><body></body>");
  const installedGlobals = installDomGlobals(browser);

  try {
    const { ScmAgentReviewViewPane } = await import("../../../../../workbench/contrib/scm/browser/scmAgentReviewViewPane.js");
    using pane = new ScmAgentReviewViewPane({ id: "zeta.gitAgentReview.test", title: "Agent Review", ownerDocument: browser.window.document });
    assert.equal(pane.element.querySelector(".zeta-scm-empty")?.textContent, "No agent changes to review.");
    const findIssues = pane.element.querySelector<HTMLButtonElement>(".zeta-scm-find-issues");
    assert.ok(findIssues);
    assert.equal(findIssues.textContent, "Find Issues");
    assert.ok(findIssues.classList.contains("zeta-button"));
    assert.ok(findIssues.classList.contains("label-centered"));
    assert.ok(findIssues.querySelector(".zeta-icon"));
  } finally {
    browser.window.close();
    for (const name of installedGlobals) Reflect.deleteProperty(globalThis, name);
  }
});

test("ScmViewPane groups App Server Git status", async () => {
  const browser = new JSDOM("<!doctype html><body></body>");
  const installedGlobals = installDomGlobals(browser);
  let requestCount = 0;
  let stagedPaths: readonly string[] | undefined;
  let committedMessage: string | undefined;
  let statusListener: ((status: GitStatus) => void) | undefined;
  const changeFileRequests: Array<{ readonly path: string; readonly comparison: "staged" | "unstaged" }> = [];
  const opened: Array<{ readonly input: EditorInput; readonly options: EditorOpenOptions | undefined }> = [];
  const first: GitStatus = {
    streamInstanceId: "git-stream-1",
    revision: 1,
    workspacePath: ".",
    head: {
      type: "branch",
      name: "main",
      objectId: "1234567890",
      upstream: { name: "origin/main", ahead: 2, behind: 1 },
    },
    changes: [
      change("staged.ts", "added", "unmodified"),
      change("src/working.ts", "unmodified", "modified"),
      change("both.ts", "modified", "modified"),
      { ...change("conflict.ts", "unmerged", "unmerged"), conflicted: true },
    ],
  };
  const committedClean: GitStatus = {
    streamInstanceId: first.streamInstanceId,
    revision: 2,
    workspacePath: first.workspacePath,
    head: first.head,
    changes: [],
  };
  const external: GitStatus = {
    ...first,
    revision: 3,
  };
  const clean: GitStatus = {
    ...committedClean,
    revision: 4,
  };
  const gitService = {
      status: async () => {
        requestCount += 1;
        return requestCount === 1 ? first : clean;
      },
      stage: async (paths: readonly string[]) => {
        stagedPaths = paths;
        return first;
      },
      unstage: async () => first,
      discardWorktree: async () => first,
      commit: async (message: string) => {
        committedMessage = message;
        return { objectId: "abcdef123456", status: committedClean };
      },
      fetch: async () => first,
      pull: async () => first,
      push: async () => first,
      changeFile: async (path: string, comparison: "staged" | "unstaged") => {
        changeFileRequests.push({ path, comparison });
        return comparison === "staged"
          ? { original: { kind: "text" as const, text: "head\n" }, modified: { kind: "text" as const, text: "index\n" } }
          : { original: { kind: "text" as const, text: "index\n" }, modified: { kind: "text" as const, text: "worktree\n" } };
      },
      onDidChangeStatus: (listener: (status: GitStatus) => void) => {
        statusListener = listener;
        const dispose = () => {
          statusListener = undefined;
        };
        return { dispose, [Symbol.dispose]: dispose };
      },
      onDidBecomeReady: () => ({ dispose(): void {}, [Symbol.dispose](): void {} }),
  } as unknown as IGitService;

  try {
    const { ScmViewPane } = await import("../../../../../workbench/contrib/scm/browser/scmViewPane.js");
    using pane = new ScmViewPane({
      id: "zeta.git",
      title: "Changes",
      ownerDocument: browser.window.document,
    }, gitService, testFileIconThemeService(), testEditorService(opened));
    browser.window.document.body.append(pane.element);
    await waitFor(() => pane.element.querySelector(".zeta-scm-status")?.textContent === "4 changed files");

    assert.equal(pane.element.querySelector(".zeta-scm-branch"), null);
    assert.equal(pane.element.querySelector(".zeta-scm-summary"), null);
    assert.ok(pane.element.querySelector(".zeta-scm-status")?.classList.contains("zeta-aria-live"));
    assert.deepEqual(
      [...pane.element.querySelectorAll(".zeta-scm-section-heading > span:first-child")].map((element) => element.textContent),
      ["Merge Changes", "Staged Changes", "Changes"],
    );
    assert.deepEqual(
      [...pane.element.querySelectorAll(".zeta-scm-section-count")].map((element) => element.textContent),
      ["1", "2", "2"],
    );

    const workingLabel = [...pane.element.querySelectorAll<HTMLElement>(".zeta-scm-change-label")]
      .find((element) => element.querySelector(".zeta-icon-label-text")?.textContent === "working.ts");
    assert.ok(workingLabel);
    assert.equal(workingLabel.querySelector(".zeta-scm-change-description")?.textContent, "src");
    assert.equal(workingLabel.querySelector(".zeta-icon-label-icon")?.getAttribute("data-file-icon"), "working.ts");
    assert.equal(pane.element.querySelector<HTMLButtonElement>('button[aria-label="Merge conflict in conflict.ts"]')?.disabled, true);

    const stagedOpen = pane.element.querySelector<HTMLButtonElement>('button[aria-label="Open staged changes for staged.ts"]');
    assert.ok(stagedOpen);
    stagedOpen.click();
    await waitFor(() => opened.length === 1);
    assert.deepEqual(changeFileRequests[0], { path: "staged.ts", comparison: "staged" });
    assert.equal(opened[0].input.contentType, "application/vnd.aster.editor-diff");
    assert.equal(opened[0].input.label, "staged.ts (HEAD) ↔ staged.ts (Index)");
    assert.equal(opened[0].options?.pinned, false);

    const workingOpen = pane.element.querySelector<HTMLButtonElement>('button[aria-label="Open changes for src/working.ts"]');
    assert.ok(workingOpen);
    workingOpen.click();
    await waitFor(() => opened.length === 2);
    assert.deepEqual(changeFileRequests[1], { path: "src/working.ts", comparison: "unstaged" });
    assert.equal(opened[1].input.contentType, "application/vnd.aster.editor-diff");
    assert.equal(opened[1].input.label, "working.ts (Index) ↔ working.ts (Working Tree)");
    assert.equal(opened[1].options?.pinned, false);

    workingOpen.dispatchEvent(new browser.window.MouseEvent("dblclick", { bubbles: true, detail: 2 }));
    await waitFor(() => opened.length === 3);
    assert.equal(opened[2].options?.pinned, true);

    const stageAll = pane.element.querySelector<HTMLButtonElement>('button[aria-label="Stage All Changes"]');
    const discardAll = pane.element.querySelector<HTMLButtonElement>('button[aria-label="Discard All Changes"]');
    assert.equal(stageAll?.dataset.icon, "add");
    assert.equal(discardAll?.dataset.icon, "discard");
    assert.ok(stageAll?.querySelector(".zeta-icon"));
    assert.ok(discardAll?.querySelector(".zeta-icon"));
    const unstageAll = pane.element.querySelector<HTMLButtonElement>('button[aria-label="Unstage All Changes"]');
    const unstage = pane.element.querySelector<HTMLButtonElement>('button[aria-label="Unstage staged.ts"]');
    assert.equal(unstageAll?.dataset.icon, "remove");
    assert.equal(unstage?.dataset.icon, "remove");
    assert.ok(unstageAll?.querySelector(".zeta-icon"));
    assert.ok(unstage?.querySelector(".zeta-icon"));

    const stage = pane.element.querySelector<HTMLButtonElement>('button[aria-label="Stage src/working.ts"]');
    const discard = pane.element.querySelector<HTMLButtonElement>('button[aria-label="Discard src/working.ts"]');
    assert.ok(stage);
    assert.equal(stage.dataset.icon, "add");
    assert.equal(discard?.dataset.icon, "discard");
    assert.ok(stage.querySelector(".zeta-icon"));
    assert.ok(discard?.querySelector(".zeta-icon"));
    stage.click();
    await waitFor(() => stagedPaths !== undefined);
    assert.deepEqual(stagedPaths, ["src/working.ts"]);

    const message = pane.element.querySelector<HTMLTextAreaElement>('[aria-label="Commit message"]');
    const commit = pane.element.querySelector<HTMLButtonElement>(".zeta-scm-commit");
    assert.ok(message);
    assert.ok(commit);
    assert.ok(commit.classList.contains("zeta-button"));
    assert.ok(commit.classList.contains("label-centered"));
    assert.equal(commit.textContent, "Commit");
    assert.ok(commit.querySelector(".zeta-icon"));
    await waitFor(() => !commit.disabled);
    message.value = "ship scm";
    commit.click();
    await waitFor(() => committedMessage !== undefined);
    assert.equal(committedMessage, "ship scm");
    await waitFor(() => pane.element.querySelector(".zeta-scm-status")?.textContent?.startsWith("Created commit abcdef1.") === true);

    assert.ok(statusListener);
    statusListener(external);
    await waitFor(() => pane.element.querySelector(".zeta-scm-status")?.textContent === "4 changed files");

    assert.equal(requestCount, 1);
  } finally {
    browser.window.close();
    for (const name of installedGlobals) Reflect.deleteProperty(globalThis, name);
  }
});

test("ScmViewPane accepts a restarted Git stream and rejects its retired predecessor", async () => {
  const browser = new JSDOM("<!doctype html><body></body>");
  const installedGlobals = installDomGlobals(browser);
  let statusRequest = 0;
  let statusListener: ((status: GitStatus) => void) | undefined;
  let readyListener: (() => void) | undefined;
  const previous: GitStatus = {
    streamInstanceId: "git-stream-before-restart",
    revision: 20,
    workspacePath: ".",
    head: { type: "branch", name: "before", objectId: "1111111", upstream: undefined },
    changes: [change("before.ts", "unmodified", "modified")],
  };
  const restarted: GitStatus = {
    streamInstanceId: "git-stream-after-restart",
    revision: 1,
    workspacePath: ".",
    head: { type: "branch", name: "after", objectId: "2222222", upstream: undefined },
    changes: [],
  };
  const latePrevious: GitStatus = {
    ...previous,
    revision: 21,
    head: { type: "branch", name: "late-before", objectId: "3333333", upstream: undefined },
  };
  const gitService = {
      status: async () => statusRequest++ === 0 ? previous : restarted,
      onDidChangeStatus: (listener: (status: GitStatus) => void) => {
        statusListener = listener;
        return { dispose(): void {}, [Symbol.dispose](): void {} };
      },
      onDidBecomeReady: (listener: () => void) => {
        readyListener = listener;
        return { dispose(): void {}, [Symbol.dispose](): void {} };
      },
  } as unknown as IGitService;

  try {
    const { ScmViewPane } = await import("../../../../../workbench/contrib/scm/browser/scmViewPane.js");
    using pane = new ScmViewPane({
      id: "zeta.git.restart",
      title: "Changes",
      ownerDocument: browser.window.document,
    }, gitService, testFileIconThemeService(), testEditorService());
    browser.window.document.body.append(pane.element);
    await waitFor(() => pane.element.querySelector('[aria-label="Open changes for before.ts"]') !== null);
    assert.equal(pane.element.querySelector(".zeta-scm-branch"), null);

    assert.ok(readyListener);
    readyListener();
    await waitFor(() => pane.element.querySelector(".zeta-scm-status")?.textContent === "No changes.");
    assert.equal(statusRequest, 2);

    assert.ok(statusListener);
    statusListener(latePrevious);
    await new Promise((resolve) => setTimeout(resolve, 0));
    assert.equal(pane.element.querySelector(".zeta-scm-status")?.textContent, "No changes.");
    assert.equal(pane.element.querySelector('[aria-label="Open changes for before.ts"]'), null);
  } finally {
    browser.window.close();
    for (const name of installedGlobals) Reflect.deleteProperty(globalThis, name);
  }
});

function change(path: string, indexStatus: GitStatus["changes"][number]["indexStatus"], worktreeStatus: GitStatus["changes"][number]["worktreeStatus"]): GitStatus["changes"][number] {
  return {
    path,
    originalPath: undefined,
    indexStatus,
    worktreeStatus,
    conflicted: false,
    submodule: {
      isSubmodule: false,
      commitChanged: false,
      trackedChanges: false,
      untrackedChanges: false,
    },
  };
}

function testManagedHover(): IManagedHover {
  return {
    visible: false,
    show() {},
    hide() {},
    update() {},
    dispose() {},
    [Symbol.dispose]() {},
  };
}

function testEditorService(opened: Array<{ readonly input: EditorInput; readonly options: EditorOpenOptions | undefined }> = []): IEditorService {
  return {
    openEditor: async (input, options) => { opened.push({ input, options }); },
    focusActiveEditor() {},
  };
}

function testFileIconThemeService(): IFileIconThemeService {
  return {
    onDidFileIconThemeChange: () => ({ dispose(): void {}, [Symbol.dispose](): void {} }),
    renderFileIcon: (resource, container) => { container.dataset.fileIcon = decodeURIComponent(resource.path.split("/").at(-1) ?? ""); },
  };
}

async function waitFor(condition: () => boolean, timeoutMillis = 1_000): Promise<void> {
  const deadline = Date.now() + timeoutMillis;
  while (!condition()) {
    if (Date.now() >= deadline) throw new Error("Timed out waiting for ScmViewPane");
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
}

function installDomGlobals(browser: JSDOM): readonly string[] {
  const globals = {
    window: browser.window,
    document: browser.window.document,
    Node: browser.window.Node,
    Element: browser.window.Element,
    HTMLElement: browser.window.HTMLElement,
    Event: browser.window.Event,
    MouseEvent: browser.window.MouseEvent,
    KeyboardEvent: browser.window.KeyboardEvent,
    navigator: browser.window.navigator,
  };
  for (const [name, value] of Object.entries(globals)) {
    Object.defineProperty(globalThis, name, { configurable: true, value });
  }
  return Object.keys(globals);
}
test("SCM presents Git unavailability without raw RPC errors", async () => {
  const { gitErrorMessage } = await import("../../../../../workbench/contrib/scm/browser/scmError.js");
  assert.equal(
    gitErrorMessage(new Error("Error invoking remote method 'zeta:git:status': JsonRpcRemoteError: GitUnavailable")),
    "Git is unavailable for this workspace. Trust the folder to enable Git changes.",
  );
});
