import { strict as assert } from "node:assert";
import test from "node:test";
import { Emitter } from "../../../../../base/common/event.js";
import { lxiconsLibrary } from "../../../../../base/common/lxiconsLibrary.js";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import type { RemoteConnectionState } from "../../../../../platform/remote/common/remote.js";
import type { RemoteAgentConnection } from "../../../../../platform/remote/common/remoteAgentApi.js";
import { RemoteStatusIndicator } from "../../browser/remoteIndicator.js";
import type { IRemoteAgentService } from "../../../../services/remote/common/remoteAgentService.js";
import { StatusbarAlignment, StatusbarService } from "../../../../services/statusbar/browser/statusbar.js";

test("remote indicator owns the leading left status entry", () => {
  using remoteAgentService = new TestRemoteAgentService("connected");
  using statusbarService = new StatusbarService();
  statusbarService.addEntry({ text: "main" }, { id: "zeta.status.git.branch", alignment: StatusbarAlignment.Left, priority: 900 });
  using indicator = new RemoteStatusIndicator({ remoteAgentService, statusbarService });

  const entries = statusbarService.getEntries(StatusbarAlignment.Left);
  assert.deepEqual(entries.map(entry => entry.id), ["zeta.status.remote", "zeta.status.git.branch"]);
  assert.equal(entries[0]?.entry.icon, lxiconsLibrary.remote);
  assert.equal(entries[0]?.entry.text, "");
  assert.equal(entries[0]?.entry.ariaLabel, "Remote connection to local backend is ready");
});

test("remote indicator projects connection changes and releases its entry", () => {
  using remoteAgentService = new TestRemoteAgentService("connecting");
  using statusbarService = new StatusbarService();
  const indicator = new RemoteStatusIndicator({ remoteAgentService, statusbarService });

  assert.equal(statusbarService.getEntries(StatusbarAlignment.Left)[0]?.entry.text, "Connecting\u2026");
  remoteAgentService.emit("reconnecting");
  assert.equal(statusbarService.getEntries(StatusbarAlignment.Left)[0]?.entry.text, "Reconnecting\u2026");
  remoteAgentService.emit("disconnected");
  assert.equal(statusbarService.getEntries(StatusbarAlignment.Left)[0]?.entry.text, "Disconnected");
  remoteAgentService.emitConnection({ kind: "ssh", generation: 2, authority: "ssh+work-server", host: "work-server" });
  assert.equal(statusbarService.getEntries(StatusbarAlignment.Left)[0]?.entry.tooltip, "SSH host work-server is disconnected");

  indicator.dispose();
  assert.deepEqual(statusbarService.getEntries(StatusbarAlignment.Left), []);
});

class TestRemoteAgentService extends DisposableOwner implements IRemoteAgentService {
  private readonly stateEmitter = this.own(new Emitter<RemoteConnectionState>());
  private readonly connectionEmitter = this.own(new Emitter<RemoteAgentConnection>());
  readonly onDidChangeConnectionState = this.stateEmitter.event;
  readonly onDidChangeConnection = this.connectionEmitter.event;
  async reconnect() { return { kind: "reconnected" } as const; }
  async rollbackRuntime() { return { kind: "rolledBack" } as const; }
  connection: RemoteAgentConnection | undefined = { kind: "local", generation: 1 };

  constructor(public connectionState: RemoteConnectionState | undefined) { super(); }

  emit(state: RemoteConnectionState): void {
    this.connectionState = state;
    this.stateEmitter.fire(state);
  }

  emitConnection(connection: RemoteAgentConnection): void {
    this.connection = connection;
    this.connectionEmitter.fire(connection);
  }
}
