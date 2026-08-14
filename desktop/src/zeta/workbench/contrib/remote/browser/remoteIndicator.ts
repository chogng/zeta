import { lxiconsLibrary } from "../../../../base/common/lxiconsLibrary.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import type { RemoteConnectionState } from "../../../../platform/remote/common/remote.js";
import type { IWorkbenchContribution } from "../../../common/contributions.js";
import type { IRemoteAgentService } from "../../../services/remote/common/remoteAgentService.js";
import type { RemoteAgentConnection } from "../../../../platform/remote/common/remoteAgentApi.js";
import { StatusbarAlignment, type IStatusbarEntry, type IStatusbarService } from "../../../services/statusbar/browser/statusbar.js";

const RemoteStatusPriority = 1_000;

export interface RemoteStatusIndicatorOptions {
  readonly remoteAgentService: IRemoteAgentService;
  readonly statusbarService: IStatusbarService;
}

/** Projects the active backend connection into the leading Workbench status item. */
export class RemoteStatusIndicator extends DisposableOwner implements IWorkbenchContribution {
  static readonly ID = "workbench.contrib.remoteStatusIndicator";

  constructor(options: RemoteStatusIndicatorOptions) {
    super();
    let state = options.remoteAgentService.connectionState ?? "connecting";
    let connection = options.remoteAgentService.connection;
    const status = this.own(options.statusbarService.addEntry(remoteStatusEntry(state, connection), {
      id: "zeta.status.remote",
      alignment: StatusbarAlignment.Left,
      priority: RemoteStatusPriority,
    }));
    this.own(options.remoteAgentService.onDidChangeConnectionState(nextState => {
      state = nextState;
      status.update(remoteStatusEntry(state, connection));
    }));
    this.own(options.remoteAgentService.onDidChangeConnection(nextConnection => {
      connection = nextConnection;
      status.update(remoteStatusEntry(state, connection));
    }));
  }
}

function remoteStatusEntry(state: RemoteConnectionState, connection: RemoteAgentConnection | undefined): IStatusbarEntry {
  const backend = connection?.kind === "ssh" ? `SSH host ${connection.host}` : "local backend";
  switch (state) {
    case "connected":
      return { icon: lxiconsLibrary.remote, text: "", ariaLabel: `Remote connection to ${backend} is ready`, tooltip: `Connected to ${backend}` };
    case "connecting":
      return { icon: lxiconsLibrary.remote, text: "Connecting\u2026", ariaLabel: `Remote connection to ${backend} is connecting`, tooltip: `Connecting to ${backend}` };
    case "reconnecting":
      return { icon: lxiconsLibrary.remote, text: "Reconnecting\u2026", ariaLabel: `Remote connection to ${backend} is reconnecting`, tooltip: `Reconnecting to ${backend}` };
    case "disconnecting":
      return { icon: lxiconsLibrary.remote, text: "Disconnecting\u2026", ariaLabel: `Remote connection to ${backend} is disconnecting`, tooltip: `Disconnecting from ${backend}` };
    case "disconnected":
      return { icon: lxiconsLibrary.remote, text: "Disconnected", ariaLabel: `Remote connection to ${backend} is disconnected`, tooltip: `${backend} is disconnected` };
  }
}
