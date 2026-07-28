import { strict as assert } from "node:assert";
import test from "node:test";
import type { AppServerSupervisor } from "../src/zeta/platform/app-server/electron-main/app-server-supervisor.js";
import { appServerIpcRoutes } from "../src/zeta/platform/app-server/electron-main/app-server-ipc.js";
import {
  registerTrustedIpcRoutes,
  type IpcMainInvokeEventLike,
  type IpcMainLike,
  type IpcRoute,
} from "../src/zeta/platform/app-server/electron-main/trusted-ipc-router.js";

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

test("App Server IPC validators reject malformed Turn, Typst, resource, and filesystem input", () => {
  const routes = appServerIpcRoutes({} as AppServerSupervisor);
  const sessionCreate = routes.find((route) => route.channel === "zeta:session:create")!;
  const turnStart = routes.find((route) => route.channel === "zeta:turn:start")!;
  const typstCompile = routes.find((route) => route.channel === "zeta:typst:compile")!;
  const resourceRead = routes.find((route) => route.channel === "zeta:resource:read")!;
  const fsGetMetadata = routes.find(
    (route) => route.channel === "zeta:fs:get-metadata",
  )!;

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
    /exactly/,
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
  }
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
