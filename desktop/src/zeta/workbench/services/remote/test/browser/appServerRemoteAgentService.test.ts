import { strict as assert } from "node:assert";
import test from "node:test";
import { Emitter } from "../../../../../base/common/event.js";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import type { AppServerConnectionState, IAppServerApi } from "../../../../../platform/app-server/common/appServerApi.js";
import type { RemoteConnectionState } from "../../../../../platform/remote/common/remote.js";
import { AppServerRemoteAgentService } from "../../browser/appServerRemoteAgentService.js";

test("remote agent events supersede a stale initial App Server read", async () => {
  using api = new TestAppServerApi();
  const states: RemoteConnectionState[] = [];
  using service = new AppServerRemoteAgentService({ api, onReadError: error => { throw error; } });
  service.onDidChangeConnectionState(state => states.push(state));

  api.emit("restarting");
  api.emit("ready");
  api.resolveInitial("starting");
  await settlePromises();

  assert.deepEqual(states, ["reconnecting", "connected"]);
  assert.equal(service.connectionState, "connected");
});

test("remote agent collapses backend states into the frontend connection lifecycle", async () => {
  using api = new TestAppServerApi();
  const states: RemoteConnectionState[] = [];
  using service = new AppServerRemoteAgentService({ api, onReadError: error => { throw error; } });
  service.onDidChangeConnectionState(state => states.push(state));

  api.resolveInitial("starting");
  await settlePromises();
  api.emit("initializing");
  api.emit("ready");
  api.emit("stopping");
  api.emit("stopped");

  assert.deepEqual(states, ["connecting", "connected", "disconnecting", "disconnected"]);
});

test("remote agent suppresses pending reads and events after disposal", async () => {
  using api = new TestAppServerApi();
  const states: RemoteConnectionState[] = [];
  const service = new AppServerRemoteAgentService({ api, onReadError: error => { throw error; } });
  service.onDidChangeConnectionState(state => states.push(state));

  await Promise.resolve();
  assert.equal(api.connectionStateReads, 1);
  service.dispose();
  api.resolveInitial("ready");
  api.emit("crashed");
  await settlePromises();

  assert.deepEqual(states, []);
});

class TestAppServerApi extends DisposableOwner implements IAppServerApi {
  private readonly stateEmitter = this.own(new Emitter<AppServerConnectionState>());
  private readonly initial = deferred<AppServerConnectionState>();
  connectionStateReads = 0;

  getConnectionState(): Promise<AppServerConnectionState> { this.connectionStateReads += 1; return this.initial.promise; }
  async getSlashCommands() { return []; }
  onConnectionState(listener: (state: AppServerConnectionState) => void) { return this.stateEmitter.event(listener); }
  emit(state: AppServerConnectionState): void { this.stateEmitter.fire(state); }
  resolveInitial(state: AppServerConnectionState): void { this.initial.resolve(state); }
}

function deferred<T>(): { readonly promise: Promise<T>; readonly resolve: (value: T) => void } {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>(accept => { resolve = accept; });
  return { promise, resolve };
}

async function settlePromises(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
  await Promise.resolve();
}
