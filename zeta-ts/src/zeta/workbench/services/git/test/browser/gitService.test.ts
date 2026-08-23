import assert from "node:assert/strict";
import test from "node:test";
import { toDisposable } from "../../../../../base/common/lifecycle.js";
import { URI } from "../../../../../base/common/uri.js";
import type { IAppServerApi, IServerEventApi } from "../../../../../platform/app-server/common/appServerApi.js";
import type { IGitApi } from "../../../../../platform/git/common/gitApi.js";
import { WorkspaceContextService } from "../../../workspaces/browser/workspaceContextService.js";
import { GitService } from "../../browser/gitService.js";

test("GitService keeps empty windows off the App Server and becomes ready with a folder", async () => {
  let statusCalls = 0;
  const api = {
    async status() {
      statusCalls += 1;
      return {
        streamInstanceId: "stream-1",
        revision: 1,
        workspacePath: "/workspace",
        head: { type: "unborn" as const, name: "main" },
        changes: [],
      };
    },
  } as unknown as IGitApi;
  const appServerApi = {
    onConnectionState: () => toDisposable(() => undefined),
  } as unknown as IAppServerApi;
  const eventApi = {
    subscribe: () => toDisposable(() => undefined),
  } as unknown as IServerEventApi;
  using workspaceContext = new WorkspaceContextService({ id: "empty-window" });
  using service = new GitService({ api, appServerApi, eventApi, workspaceContext });
  let readyEvents = 0;
  using ready = service.onDidBecomeReady(() => readyEvents += 1);

  await assert.rejects(service.status(), /GitUnavailable/);
  assert.equal(statusCalls, 0);

  workspaceContext.updateWorkspace({ id: "workspace", uri: URI.file("/workspace") });
  assert.equal(readyEvents, 1);
  assert.equal((await service.status()).workspacePath, "/workspace");
  assert.equal(statusCalls, 1);
});
