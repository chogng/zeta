import { registerWorkbenchContribution, WorkbenchPhase } from "../../../common/contributions.js";
import { IExtensionService } from "../../../services/extensions/common/extensionService.js";
import { IRemoteAgentService } from "../../../services/remote/common/remoteAgentService.js";
import { IStatusbarService } from "../../../services/statusbar/browser/statusbar.js";
import { RemoteExtensionRecoveryContribution } from "./remoteExtensionRecovery.js";
import { RemoteStatusIndicator } from "./remoteIndicator.js";

registerWorkbenchContribution(RemoteStatusIndicator.ID, WorkbenchPhase.BlockStartup, accessor => new RemoteStatusIndicator({
  remoteAgentService: accessor.get(IRemoteAgentService),
  statusbarService: accessor.get(IStatusbarService),
}));

registerWorkbenchContribution(RemoteExtensionRecoveryContribution.ID, WorkbenchPhase.BlockStartup, accessor => new RemoteExtensionRecoveryContribution({
  extensionService: accessor.get(IExtensionService),
  remoteAgentService: accessor.get(IRemoteAgentService),
}));
