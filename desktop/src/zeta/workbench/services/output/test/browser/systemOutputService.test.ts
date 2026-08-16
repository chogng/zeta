import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import type { AppServerConnectionState, IAppServerApi } from "../../../../../platform/app-server/common/appServerApi.js";
import { OutputService } from "../../browser/outputService.js";
import { SystemOutputService } from "../../browser/systemOutputService.js";

test("SystemOutputService projects App Server lifecycle and window errors", async () => {
  const browser = new JSDOM("<!doctype html><body><main></main></body>", { url: "https://zeta.test" });
  const listeners = new Set<(state: AppServerConnectionState) => void>();
  const appServer: IAppServerApi = {
    getConnectionState: async () => "ready",
    getSlashCommands: async () => [],
    onConnectionState: listener => { listeners.add(listener); return { dispose: () => listeners.delete(listener) }; },
  };
  using output = new OutputService();
  using service = new SystemOutputService(output, appServer, { root: browser.window.document.querySelector("main")! });
  await Promise.resolve();
  for (const listener of listeners) listener("crashed");
  browser.window.dispatchEvent(new browser.window.ErrorEvent("error", { message: "render failed", filename: "file:///workspace/main.ts", lineno: 3, colno: 4 }));

  assert.match(output.getChannel("app-server")?.getText() ?? "", /Initial App Server connection state: ready/);
  assert.match(output.getChannel("app-server")?.getText() ?? "", /connection is crashed/);
  assert.match(output.getChannel("window")?.getText() ?? "", /render failed.*main\.ts:3:4/);
  browser.window.close();
});
