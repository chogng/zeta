import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { AnchorAxisAlignment, AnchorPosition } from "../../../../../base/common/layout.js";
import type { IContextMenuService } from "../../../../../platform/contextview/browser/contextMenu.js";
import type { HoverSetupOptions, IHoverService, IManagedHover } from "../../../../../platform/hover/common/hoverService.js";
import type { GitStatus, IGitService } from "../../../../../workbench/services/git/common/gitService.js";

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

test("ScmGraphViewPane renders bounded repository history", async () => {
  const browser = new JSDOM("<!doctype html><body></body>");
  const installedGlobals = installDomGlobals(browser);
  let historyRequests = 0;
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
      history: async () => {
        historyRequests += 1;
        return [
          { objectId: "1234567890abcdef", parentObjectIds: ["abcdef1234567890", "side-parent"], timestampSeconds: 1_753_000_000, subject: "Wire SCM panes" },
          { objectId: "abcdef1234567890", parentObjectIds: ["parent-one", "parent-two"], timestampSeconds: 1_752_900_000, subject: "Prepare graph data" },
        ];
      },
  } as unknown as IGitService;

  try {
    const { ScmGraphViewPane } = await import("../../../../../workbench/contrib/scm/browser/scmGraphViewPane.js");
    using pane = new ScmGraphViewPane({ id: "zeta.gitGraph.test", title: "Graph", ownerDocument: browser.window.document }, gitService, menuService, {} as IContextMenuService, contextKeyService, hoverService);
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
    await waitFor(() => historyRequests === 2 && pane.element.querySelectorAll(".zeta-scm-graph-subject").length === 2);

    assert.deepEqual([...pane.element.querySelectorAll(".zeta-scm-graph-subject")].map((element) => element.textContent), ["Wire SCM panes", "Prepare graph data"]);
    assert.equal(pane.element.querySelector(".zeta-scm-graph-commit.current")?.getAttribute("aria-current"), "true");
    assert.ok(pane.element.querySelector(".zeta-scm-graph-commit.head"));
    assert.ok(pane.element.querySelector(".zeta-scm-graph-commit.merge"));
    assert.ok(pane.element.querySelector(".zeta-scm-graph-commit.head.merge"));
    assert.equal(pane.element.querySelector(".zeta-scm-graph-head")?.textContent, "main");
    assert.ok(pane.element.querySelector(".zeta-scm-graph-head .zeta-icon"));
    assert.equal(hoverOptions.length, 4);
    assert.ok(hoverOptions.every((options) => options.target.classList.contains("zeta-scm-graph-commit")));
    assert.ok(hoverOptions.every((options) => options.anchorAxisAlignment === AnchorAxisAlignment.Horizontal));
    assert.ok(hoverOptions.every((options) => options.anchorPosition === AnchorPosition.Below));
    assert.equal(pane.element.querySelector(".zeta-scm-graph-graph.head")?.querySelectorAll(".zeta-scm-graph-node").length, 2);
    assert.equal(pane.element.querySelector(".zeta-scm-graph-commit.head.merge > .zeta-scm-graph-graph")?.classList.contains("head"), true);
    assert.equal(pane.element.querySelector(".zeta-scm-graph-commit.head.merge > .zeta-scm-graph-graph")?.classList.contains("merge"), false);
    assert.equal(pane.element.querySelector(".zeta-scm-graph-graph.merge")?.querySelectorAll(".zeta-scm-graph-node").length, 2);
    assert.equal(pane.element.querySelector<SVGSVGElement>(".zeta-scm-graph-graph.merge")?.style.width, "44px");
    assert.ok((pane.element.querySelector(".zeta-scm-graph-graph.merge")?.querySelectorAll(".zeta-scm-graph-path").length ?? 0) > 1);
    assert.match(pane.element.querySelector(".zeta-scm-graph-metadata")?.textContent ?? "", /^1234567 · /);
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
      change("working.ts", "unmodified", "modified"),
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
    }, gitService);
    browser.window.document.body.append(pane.element);
    await waitFor(() => pane.element.querySelector(".zeta-scm-status")?.textContent === "4 changed files");

    assert.equal(pane.element.querySelector(".zeta-scm-branch")?.textContent, "main ↑2 ↓1");
    assert.deepEqual(
      [...pane.element.querySelectorAll(".zeta-scm-section-heading > span:first-child")].map((element) => element.textContent),
      ["Merge Changes", "Staged Changes", "Changes"],
    );
    assert.deepEqual(
      [...pane.element.querySelectorAll(".zeta-scm-section-count")].map((element) => element.textContent),
      ["1", "2", "2"],
    );

    const stage = pane.element.querySelector<HTMLButtonElement>('button[aria-label="Stage working.ts"]');
    assert.ok(stage);
    stage.click();
    await waitFor(() => stagedPaths !== undefined);
    assert.deepEqual(stagedPaths, ["working.ts"]);

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
    }, gitService);
    browser.window.document.body.append(pane.element);
    await waitFor(() => pane.element.querySelector(".zeta-scm-branch")?.textContent === "before");

    assert.ok(readyListener);
    readyListener();
    await waitFor(() => pane.element.querySelector(".zeta-scm-branch")?.textContent === "after");
    assert.equal(statusRequest, 2);

    assert.ok(statusListener);
    statusListener(latePrevious);
    await new Promise((resolve) => setTimeout(resolve, 0));
    assert.equal(pane.element.querySelector(".zeta-scm-branch")?.textContent, "after");
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
