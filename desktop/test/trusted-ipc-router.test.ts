import { strict as assert } from "node:assert";
import test from "node:test";
import type { AppServerSupervisor } from "../src/platform/app-server/electron-main/app-server-supervisor.js";
import { appServerIpcRoutes } from "../src/platform/app-server/electron-main/app-server-ipc.js";
import {
  registerTrustedIpcRoutes,
  type IpcMainInvokeEventLike,
  type IpcMainLike,
  type IpcRoute,
} from "../src/platform/app-server/electron-main/trusted-ipc-router.js";

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

  dispose();
  assert.equal(ipcMain.handlers.size, 0);
});

test("App Server IPC validators reject unknown fields and malformed Turn input", () => {
  const routes = appServerIpcRoutes({} as AppServerSupervisor);
  const sessionCreate = routes.find((route) => route.channel === "zeta:session:create")!;
  const turnStart = routes.find((route) => route.channel === "zeta:turn:start")!;

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
