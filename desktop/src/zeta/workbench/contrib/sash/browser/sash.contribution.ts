import { IConfigurationService } from "../../../../platform/configuration/common/configuration.js";
import { registerWorkbenchContribution, WorkbenchPhase } from "../../../common/contributions.js";
import { IWorkbenchWindowService } from "../../../browser/window.js";
import { SashSettingsController } from "./sash.js";

registerWorkbenchContribution(
  "workbench.contrib.sash",
  WorkbenchPhase.AfterRestored,
  (accessor) => new SashSettingsController(
    accessor.get(IConfigurationService),
    accessor.get(IWorkbenchWindowService).root,
  ),
);
