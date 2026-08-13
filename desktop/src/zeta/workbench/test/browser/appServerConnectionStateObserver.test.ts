import { strict as assert } from "node:assert";
import test from "node:test";
import { Emitter } from "../../../base/common/event.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import type { AppServerConnectionState, IAppServerApi } from "../../../platform/app-server/common/appServerApi.js";
import { AppServerConnectionStateObserver } from "../../browser/appServerConnectionStateObserver.js";

test("App Server connection observation ignores an initial read superseded by events", async () => {
  using api = new TestAppServerApi();
  const states: AppServerConnectionState[] = [];
  using observer = new AppServerConnectionStateObserver({ api, onState: state => states.push(state), onReadError: error => { throw error; } });

  api.emit("restarting");
  api.emit("ready");
  api.resolveInitial("starting");
  await settlePromises();

  assert.deepEqual(states, ["restarting", "ready"]);
});

test("App Server connection observation suppresses pending reads and events after disposal", async () => {
  using api = new TestAppServerApi();
  const states: AppServerConnectionState[] = [];
  const errors: unknown[] = [];
  const observer = new AppServerConnectionStateObserver({ api, onState: state => states.push(state), onReadError: error => errors.push(error) });

  await Promise.resolve();
  assert.equal(api.connectionStateReads, 1);
  observer.dispose();
  api.resolveInitial("ready");
  api.emit("crashed");
  await settlePromises();

  assert.deepEqual(states, []);
  assert.deepEqual(errors, []);
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
