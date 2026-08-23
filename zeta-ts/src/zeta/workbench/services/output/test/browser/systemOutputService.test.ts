import assert from "node:assert/strict";
import test from "node:test";
import { Emitter } from "../../../../../base/common/event.js";
import type { AppServerConnectionState, IAppServerApi } from "../../../../../platform/app-server/common/appServerApi.js";
import type { IWorkbenchHostService, WorkbenchHostError } from "../../../host/common/workbenchHostService.js";
import { OutputService } from "../../browser/outputService.js";
import { SystemOutputService } from "../../browser/systemOutputService.js";

test("SystemOutputService projects App Server lifecycle and window errors", async () => {
  const listeners = new Set<(state: AppServerConnectionState) => void>();
  using hostErrors = new Emitter<WorkbenchHostError>();
  const host: IWorkbenchHostService = { onDidError: hostErrors.event, downloadText: () => { throw new Error("Unexpected download"); } };
  const appServer: IAppServerApi = {
    getConnectionState: async () => "ready",
    getSlashCommands: async () => [],
    onConnectionState: listener => { listeners.add(listener); return { dispose: () => listeners.delete(listener) }; },
  };
  using output = new OutputService();
  using service = new SystemOutputService(output, appServer, host);
  await Promise.resolve();
  for (const listener of listeners) listener("crashed");
  hostErrors.fire({ kind: "runtimeError", message: "render failed", source: "file:///workspace/main.ts:3:4" });

  assert.match(output.getChannel("app-server")?.getText() ?? "", /Initial App Server connection state: ready/);
  assert.match(output.getChannel("app-server")?.getText() ?? "", /connection is crashed/);
  assert.match(output.getChannel("window")?.getText() ?? "", /render failed.*main\.ts:3:4/);
});
