import { strict as assert } from "node:assert";
import test from "node:test";
import { Emitter } from "../../../../../base/common/event.js";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import type { RemoteConnectionState } from "../../../../../platform/remote/common/remote.js";
import { RemoteExtensionRecoveryContribution } from "../../browser/remoteExtensionRecovery.js";
import type { IExtensionService } from "../../../../services/extensions/common/extensionService.js";
import type { IRemoteAgentService } from "../../../../services/remote/common/remoteAgentService.js";

test("remote extension recovery reloads only after a known non-connected state", async () => {
  using remoteAgentService = new TestRemoteAgentService();
  let reloads = 0;
  const extensionService = { reload: async () => { reloads += 1; } } as unknown as IExtensionService;
  using contribution = new RemoteExtensionRecoveryContribution({ extensionService, remoteAgentService });

  remoteAgentService.emit("connected");
  remoteAgentService.emit("reconnecting");
  remoteAgentService.emit("connected");
  remoteAgentService.emit("connected");
  await Promise.resolve();

  assert.equal(reloads, 1);
});

class TestRemoteAgentService extends DisposableOwner implements IRemoteAgentService {
  private readonly stateEmitter = this.own(new Emitter<RemoteConnectionState>());
  connectionState: RemoteConnectionState | undefined;
  readonly onDidChangeConnectionState = this.stateEmitter.event;

  emit(state: RemoteConnectionState): void {
    this.connectionState = state;
    this.stateEmitter.fire(state);
  }
}
