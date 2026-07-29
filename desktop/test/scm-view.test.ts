import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import type { GitStatusResult, ServerNotification } from "../generated/app-server/types.js";
import type { ZetaRendererApi } from "../src/zeta/platform/app-server/common/renderer-api.js";

test("ScmViewPane groups App Server Git status and refreshes it", async () => {
  const browser = new JSDOM("<!doctype html><body></body>");
  const installedGlobals = installDomGlobals(browser);
  let requestCount = 0;
  let stagedPaths: readonly string[] | undefined;
  let committedMessage: string | undefined;
  let notificationListener: ((event: ServerNotification) => void) | undefined;
  const first: GitStatusResult = {
    streamInstanceId: "git-stream-1",
    revision: 1,
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
  const committedClean: GitStatusResult = {
    streamInstanceId: first.streamInstanceId,
    revision: 2,
    head: first.head,
    changes: [],
  };
  const external: GitStatusResult = {
    ...first,
    revision: 3,
  };
  const clean: GitStatusResult = {
    ...committedClean,
    revision: 4,
  };
  const api = {
    git: {
      status: async () => {
        requestCount += 1;
        return requestCount === 1 ? first : clean;
      },
      stage: async (params: { paths: string[] }) => {
        stagedPaths = params.paths;
        return { status: first };
      },
      unstage: async () => ({ status: first }),
      discardWorktree: async () => ({ status: first }),
      commit: async (params: { message: string }) => {
        committedMessage = params.message;
        return { objectId: "abcdef123456", status: committedClean };
      },
      fetch: async () => ({ status: first }),
      pull: async () => ({ status: first }),
      push: async () => ({ status: first }),
    },
    events: {
      subscribe: (listener: (event: ServerNotification) => void) => {
        notificationListener = listener;
        return { dispose: () => {
          notificationListener = undefined;
        } };
      },
    },
    appServer: {
      onConnectionState: () => ({ dispose(): void {} }),
    },
  } as unknown as ZetaRendererApi;

  try {
    const { ScmViewPane } = await import("../src/zeta/workbench/contrib/scm/browser/scmViewPane.js");
    using pane = new ScmViewPane({
      id: "zeta.git",
      title: "Changes",
      ownerDocument: browser.window.document,
    }, api);
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
    await waitFor(() => !commit.disabled);
    message.value = "ship scm";
    commit.click();
    await waitFor(() => committedMessage !== undefined);
    assert.equal(committedMessage, "ship scm");
    await waitFor(() => pane.element.querySelector(".zeta-scm-status")?.textContent?.startsWith("Created commit abcdef1.") === true);

    assert.ok(notificationListener);
    notificationListener({ method: "git/statusChanged", params: { status: external } });
    await waitFor(() => pane.element.querySelector(".zeta-scm-status")?.textContent === "4 changed files");

    const refresh = pane.element.querySelector<HTMLButtonElement>(".zeta-scm-refresh");
    assert.ok(refresh);
    refresh.click();
    await waitFor(() => pane.element.querySelector(".zeta-scm-status")?.textContent === "No changes.");
    assert.equal(requestCount, 2);
    assert.equal(pane.element.querySelectorAll(".zeta-scm-section").length, 0);
  } finally {
    browser.window.close();
    for (const name of installedGlobals) Reflect.deleteProperty(globalThis, name);
  }
});

test("ScmViewPane accepts a restarted Git stream and rejects its retired predecessor", async () => {
  const browser = new JSDOM("<!doctype html><body></body>");
  const installedGlobals = installDomGlobals(browser);
  let statusRequest = 0;
  let notificationListener: ((event: ServerNotification) => void) | undefined;
  let connectionListener: ((state: "ready") => void) | undefined;
  const previous: GitStatusResult = {
    streamInstanceId: "git-stream-before-restart",
    revision: 20,
    head: { type: "branch", name: "before", objectId: "1111111", upstream: null },
    changes: [change("before.ts", "unmodified", "modified")],
  };
  const restarted: GitStatusResult = {
    streamInstanceId: "git-stream-after-restart",
    revision: 1,
    head: { type: "branch", name: "after", objectId: "2222222", upstream: null },
    changes: [],
  };
  const latePrevious: GitStatusResult = {
    ...previous,
    revision: 21,
    head: { type: "branch", name: "late-before", objectId: "3333333", upstream: null },
  };
  const api = {
    git: {
      status: async () => statusRequest++ === 0 ? previous : restarted,
    },
    events: {
      subscribe: (listener: (event: ServerNotification) => void) => {
        notificationListener = listener;
        return { dispose(): void {} };
      },
    },
    appServer: {
      onConnectionState: (listener: (state: "ready") => void) => {
        connectionListener = listener;
        return { dispose(): void {} };
      },
    },
  } as unknown as ZetaRendererApi;

  try {
    const { ScmViewPane } = await import("../src/zeta/workbench/contrib/scm/browser/scmViewPane.js");
    using pane = new ScmViewPane({
      id: "zeta.git.restart",
      title: "Changes",
      ownerDocument: browser.window.document,
    }, api);
    browser.window.document.body.append(pane.element);
    await waitFor(() => pane.element.querySelector(".zeta-scm-branch")?.textContent === "before");

    assert.ok(connectionListener);
    connectionListener("ready");
    await waitFor(() => pane.element.querySelector(".zeta-scm-branch")?.textContent === "after");
    assert.equal(statusRequest, 2);

    assert.ok(notificationListener);
    notificationListener({ method: "git/statusChanged", params: { status: latePrevious } });
    await new Promise((resolve) => setTimeout(resolve, 0));
    assert.equal(pane.element.querySelector(".zeta-scm-branch")?.textContent, "after");
  } finally {
    browser.window.close();
    for (const name of installedGlobals) Reflect.deleteProperty(globalThis, name);
  }
});

function change(path: string, indexStatus: GitStatusResult["changes"][number]["indexStatus"], worktreeStatus: GitStatusResult["changes"][number]["worktreeStatus"]): GitStatusResult["changes"][number] {
  return {
    path,
    originalPath: null,
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
