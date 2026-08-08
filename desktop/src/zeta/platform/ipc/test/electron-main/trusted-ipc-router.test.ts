import { strict as assert } from "node:assert";
import test from "node:test";
import type { AppServerSupervisor } from "../../../../platform/app-server/electron-main/app-server-supervisor.js";
import { appServerIpcRoutes } from "../../../../platform/app-server/electron-main/app-server-ipc.js";
import { registerTrustedIpcRoutes, type IpcMainInvokeEventLike, type IpcMainLike, type IpcRoute } from "../../../../platform/ipc/electron-main/trustedIpcRouter.js";
import { fileIpcRoutes } from "../../../../platform/files/electron-main/fileIpcRoutes.js";
import { gitIpcRoutes } from "../../../../platform/git/electron-main/gitIpcRoutes.js";
import { searchIpcRoutes } from "../../../../platform/search/electron-main/searchIpcRoutes.js";
import { sessionIpcRoutes } from "../../../../platform/sessions/electron-main/sessionIpcRoutes.js";
import { terminalIpcRoutes } from "../../../../platform/terminal/electron-main/terminalIpcRoutes.js";
import { typstIpcRoutes } from "../../../../platform/typst/electron-main/typstIpcRoutes.js";

class FakeIpcMain implements IpcMainLike {
  readonly handlers = new Map<
    string,
    (event: IpcMainInvokeEventLike, params: unknown) => unknown
  >();

  handle(
    channel: string,
    listener: (event: IpcMainInvokeEventLike, params: unknown) => unknown,
  ): void {
    this.handlers.set(channel, listener);
  }

  removeHandler(channel: string): void {
    this.handlers.delete(channel);
  }
}

function target(url = "file:///app/workbench.html") {
  const mainFrame = { url };
  const webContents = { mainFrame };
  return {
    mainFrame,
    webContents,
    event: { sender: webContents, senderFrame: mainFrame },
  };
}

test("trusted IPC router enforces webContents, main frame, exact URL, and params", async () => {
  const ipcMain = new FakeIpcMain();
  const trusted = target();
  let calls = 0;
  const routes: readonly IpcRoute<unknown, unknown>[] = [
    {
      channel: "test:invoke",
      validate(value) {
        if (value !== "valid") throw new Error("invalid params");
        return value;
      },
      invoke() {
        calls += 1;
        return "ok";
      },
    },
  ];
  const dispose = registerTrustedIpcRoutes(
    ipcMain,
    {
      webContents: trusted.webContents,
      allowedEntryUrls: new Set(["file:///app/workbench.html"]),
    },
    routes,
  );
  const invoke = ipcMain.handlers.get("test:invoke")!;

  assert.equal(await invoke(trusted.event, "valid"), "ok");
  assert.throws(
    () =>
      invoke(
        { sender: { mainFrame: trusted.mainFrame }, senderFrame: trusted.mainFrame },
        "valid",
      ),
    /Untrusted renderer/,
  );
  assert.throws(
    () =>
      invoke(
        {
          sender: trusted.webContents,
          senderFrame: { url: "file:///app/workbench.html" },
        },
        "valid",
      ),
    /main frame/,
  );
  trusted.mainFrame.url = "file:///app/other.html";
  assert.throws(() => invoke(trusted.event, "valid"), /URL is not allowed/);
  trusted.mainFrame.url = "file:///app/workbench.html";
  assert.throws(() => invoke(trusted.event, "invalid"), /invalid params/);
  assert.equal(calls, 1);

  dispose.dispose();
  assert.equal(ipcMain.handlers.size, 0);
});

test("capability IPC validators reject malformed input", () => {
  const supervisor = {} as AppServerSupervisor;
  const routes = [
    ...appServerIpcRoutes(supervisor),
    ...sessionIpcRoutes(supervisor),
    ...typstIpcRoutes(supervisor),
    ...fileIpcRoutes(supervisor),
    ...gitIpcRoutes(supervisor),
    ...searchIpcRoutes(supervisor),
    ...terminalIpcRoutes(supervisor),
  ];
  const sessionCreate = routes.find((route) => route.channel === "zeta:session:create")!;
  const turnStart = routes.find((route) => route.channel === "zeta:turn:start")!;
  const resolveInteraction = routes.find(
    (route) => route.channel === "zeta:turn:interaction:resolve",
  )!;
  const typstCompile = routes.find((route) => route.channel === "zeta:typst:compile")!;
  const resourceRead = routes.find((route) => route.channel === "zeta:resource:read")!;
  const fsGetMetadata = routes.find(
    (route) => route.channel === "zeta:fs:get-metadata",
  )!;
  const fsReadFile = routes.find(
    (route) => route.channel === "zeta:fs:read-file",
  )!;
  const fsReadBinaryFile = routes.find(
    (route) => route.channel === "zeta:fs:read-binary-file",
  )!;
  const searchStart = routes.find(
    (route) => route.channel === "zeta:workspace-search:start",
  )!;
  const searchRead = routes.find(
    (route) => route.channel === "zeta:workspace-search:read",
  )!;
  const terminalCreate = routes.find(
    (route) => route.channel === "zeta:terminal:create",
  )!;
  const terminalProfileList = routes.find(
    (route) => route.channel === "zeta:terminal:profile-list",
  )!;
  const terminalWrite = routes.find(
    (route) => route.channel === "zeta:terminal:write",
  )!;
  const terminalResize = routes.find(
    (route) => route.channel === "zeta:terminal:resize",
  )!;
  const terminalRead = routes.find(
    (route) => route.channel === "zeta:terminal:read",
  )!;
  const terminalClose = routes.find(
    (route) => route.channel === "zeta:terminal:close",
  )!;
  const gitStage = routes.find((route) => route.channel === "zeta:git:stage")!;
  const gitCommit = routes.find((route) => route.channel === "zeta:git:commit")!;

  assert.deepEqual(
    sessionCreate.validate({ commandId: "one", title: "title" }),
    { commandId: "one", title: "title" },
  );
  assert.throws(
    () =>
      sessionCreate.validate({
        commandId: "one",
        title: "title",
        unexpected: true,
      }),
    /only optional keys/,
  );
  assert.throws(
    () =>
      turnStart.validate({
        commandId: "one",
        sessionId: "session_1",
        threadId: "thread_1",
        expectedSequence: 1,
        input: [],
      }),
    /non-empty array/,
  );
  assert.throws(
    () =>
      turnStart.validate({
        commandId: "one",
        sessionId: "session_1",
        threadId: "thread_1",
        expectedSequence: 1,
        input: [{ type: "image", text: "no" }],
      }),
    /must be text/,
  );
  assert.deepEqual(resolveInteraction.validate({
    commandId: "resolve-1",
    sessionId: "session_1",
    threadId: "thread_1",
    turnId: "turn_1",
    requestId: "request_1",
    expectedSequence: 3,
    response: {
      type: "approval",
      response: { decision: "approveOnce" },
    },
  }), {
    commandId: "resolve-1",
    sessionId: "session_1",
    threadId: "thread_1",
    turnId: "turn_1",
    requestId: "request_1",
    expectedSequence: 3,
    response: {
      type: "approval",
      response: { decision: "approveOnce" },
    },
  });
  assert.throws(
    () => resolveInteraction.validate({
      commandId: "resolve-1",
      sessionId: "session_1",
      threadId: "thread_1",
      turnId: "turn_1",
      requestId: "request_1",
      expectedSequence: 3,
      response: {
        type: "approval",
        response: { decision: "always" },
      },
    }),
    /response.decision/,
  );
  assert.deepEqual(typstCompile.validate({ source: "= Paper" }), {
    source: "= Paper",
  });
  assert.throws(
    () => typstCompile.validate({ source: "a".repeat(1024 * 1024 + 1) }),
    /UTF-8 bytes/,
  );
  assert.throws(
    () =>
      resourceRead.validate({
        resourceId: "resource_1",
        offset: 0,
        maxBytes: 262_145,
      }),
    /must not exceed/,
  );
  assert.deepEqual(fsGetMetadata.validate({ path: "src/main.ts" }), {
    path: "src/main.ts",
  });
  assert.deepEqual(fsReadFile.validate({ path: "src/main.ts" }), {
    path: "src/main.ts",
  });
  assert.deepEqual(fsReadBinaryFile.validate({ path: "paper.pdf" }), {
    path: "paper.pdf",
  });
  for (const path of [
    "../outside",
    "src/../../outside",
    "/absolute",
    "\\\\server\\share",
    "C:\\absolute",
    "src\0file",
  ]) {
    assert.throws(
      () => fsGetMetadata.validate({ path }),
      /relative to the workspace root/,
    );
    assert.throws(
      () => fsReadFile.validate({ path }),
      /relative to the workspace root/,
    );
    assert.throws(
      () => fsReadBinaryFile.validate({ path }),
      /relative to the workspace root/,
    );
  }
  assert.deepEqual(searchStart.validate({
    query: "needle",
    patternKind: "literal",
    caseSensitivity: "smart",
    includePatterns: ["src/**"],
    excludePatterns: ["**/*.test.ts"],
    maxResults: 2_000,
  }), {
    query: "needle",
    patternKind: "literal",
    caseSensitivity: "smart",
    includePatterns: ["src/**"],
    excludePatterns: ["**/*.test.ts"],
    maxResults: 2_000,
  });
  for (const includePatterns of [
    ["../outside"],
    ["!src/**"],
    ["/absolute"],
    ["C:\\absolute"],
  ]) {
    assert.throws(() => searchStart.validate({
      query: "needle",
      patternKind: "literal",
      caseSensitivity: "smart",
      includePatterns,
      excludePatterns: [],
      maxResults: 2_000,
    }), /workspace-relative glob/);
  }
  assert.throws(() => searchStart.validate({
    query: "needle",
    patternKind: "glob",
    caseSensitivity: "smart",
    includePatterns: [],
    excludePatterns: [],
    maxResults: 2_000,
  }), /patternKind/);
  assert.deepEqual(searchRead.validate({
    searchId: "search-1",
    afterMatch: 0,
    maxMatches: 100,
  }), {
    searchId: "search-1",
    afterMatch: 0,
    maxMatches: 100,
  });
  assert.throws(() => searchRead.validate({
    searchId: "search-1",
    afterMatch: 0,
    maxMatches: 201,
  }), /must not exceed 200/);
  assert.deepEqual(terminalCreate.validate({
    rows: 24,
    cols: 80,
    profile: { type: "default" },
  }), {
    rows: 24,
    cols: 80,
    profile: { type: "default" },
  });
  assert.throws(() => terminalCreate.validate({
    rows: 0,
    cols: 80,
    profile: { type: "default" },
  }), /must be positive/);
  assert.deepEqual(terminalCreate.validate({
    rows: 24,
    cols: 80,
    profile: { type: "profile", profileId: "powershell" },
  }), {
    rows: 24,
    cols: 80,
    profile: { type: "profile", profileId: "powershell" },
  });
  assert.throws(() => terminalCreate.validate({
    rows: 24,
    cols: 80,
    profile: { type: "profile", executable: "cmd.exe" },
  }), /profile/);
  assert.deepEqual(terminalProfileList.validate(undefined), {});
  assert.deepEqual(terminalWrite.validate({
    terminalId: "terminal-1",
    data: "echo hello\r",
  }), {
    terminalId: "terminal-1",
    data: "echo hello\r",
  });
  assert.deepEqual(terminalWrite.validate({
    terminalId: "terminal-1",
    data: " ",
  }), {
    terminalId: "terminal-1",
    data: " ",
  });
  assert.throws(() => terminalWrite.validate({
    terminalId: "terminal-1",
    data: "a".repeat(65_537),
  }), /UTF-8 bytes/);
  assert.deepEqual(terminalResize.validate({
    terminalId: "terminal-1",
    rows: 40,
    cols: 120,
  }), {
    terminalId: "terminal-1",
    rows: 40,
    cols: 120,
  });
  assert.throws(() => terminalResize.validate({
    terminalId: "terminal-1",
    rows: 40,
    cols: 513,
  }), /must not exceed 512/);
  assert.deepEqual(terminalRead.validate({
    terminalId: "terminal-1",
    afterSequence: 0,
    afterCommandSequence: 0,
    maxChunks: 128,
  }), {
    terminalId: "terminal-1",
    afterSequence: 0,
    afterCommandSequence: 0,
    maxChunks: 128,
  });
  assert.throws(() => terminalRead.validate({
    terminalId: "terminal-1",
    afterSequence: 0,
    afterCommandSequence: 0,
    maxChunks: 129,
  }), /must not exceed 128/);
  assert.deepEqual(terminalClose.validate({
    terminalId: "terminal-1",
  }), {
    terminalId: "terminal-1",
  });
  assert.throws(() => terminalClose.validate({
    terminalId: "terminal-1",
    force: true,
  }), /exactly/);
  assert.deepEqual(gitStage.validate({ paths: ["src/main.ts", "README.md"] }), {
    paths: ["src/main.ts", "README.md"],
  });
  for (const paths of [[], ["../outside"], ["/absolute"], ["C:\\absolute"], ["src\0file"]]) {
    assert.throws(() => gitStage.validate({ paths }), /paths|workspace root/);
  }
  assert.deepEqual(gitCommit.validate({ message: "feat: add SCM actions" }), {
    message: "feat: add SCM actions",
  });
  assert.throws(() => gitCommit.validate({ message: "   " }), /non-empty/);
  assert.throws(() => gitCommit.validate({ message: "bad\0message" }), /NUL/);
  assert.throws(() => gitCommit.validate({ message: "a".repeat(65_537) }), /UTF-8 bytes/);
});

test("trusted IPC router rejects duplicate route registrations", () => {
  const ipcMain = new FakeIpcMain();
  const trusted = target();
  const route: IpcRoute<unknown, unknown> = {
    channel: "duplicate",
    validate: (value) => value,
    invoke: () => undefined,
  };

  assert.throws(
    () =>
      registerTrustedIpcRoutes(
        ipcMain,
        {
          webContents: trusted.webContents,
          allowedEntryUrls: new Set(["file:///app/workbench.html"]),
        },
        [route, route],
      ),
    /Duplicate IPC route/,
  );
  assert.equal(ipcMain.handlers.size, 0);
});
