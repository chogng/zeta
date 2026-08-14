import { lxiconsLibrary } from "../../../../base/common/lxiconsLibrary.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import type { RemoteConnectionState } from "../../../../platform/remote/common/remote.js";
import type { IWorkbenchContribution } from "../../../common/contributions.js";
import type { IRemoteAgentService } from "../../../services/remote/common/remoteAgentService.js";
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
    const status = this.own(options.statusbarService.addEntry(remoteStatusEntry(options.remoteAgentService.connectionState ?? "connecting"), {
      id: "zeta.status.remote",
      alignment: StatusbarAlignment.Left,
      priority: RemoteStatusPriority,
    }));
    this.own(options.remoteAgentService.onDidChangeConnectionState(state => status.update(remoteStatusEntry(state))));
  }
}

function remoteStatusEntry(state: RemoteConnectionState): IStatusbarEntry {
  switch (state) {
    case "connected":
      return { icon: lxiconsLibrary.remote, text: "", ariaLabel: "Remote connection is ready", tooltip: "Connected to remote backend" };
    case "connecting":
      return { icon: lxiconsLibrary.remote, text: "Connecting\u2026", ariaLabel: "Remote connection is connecting", tooltip: "Connecting to remote backend" };
    case "reconnecting":
      return { icon: lxiconsLibrary.remote, text: "Reconnecting\u2026", ariaLabel: "Remote connection is reconnecting", tooltip: "Reconnecting to remote backend" };
    case "disconnecting":
      return { icon: lxiconsLibrary.remote, text: "Disconnecting\u2026", ariaLabel: "Remote connection is disconnecting", tooltip: "Disconnecting from remote backend" };
    case "disconnected":
      return { icon: lxiconsLibrary.remote, text: "Disconnected", ariaLabel: "Remote connection is disconnected", tooltip: "Remote backend is disconnected" };
  }
}
