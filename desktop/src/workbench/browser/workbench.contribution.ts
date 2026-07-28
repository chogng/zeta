import {
  IKeybindingsResourceService,
} from "../../platform/keybinding/common/keybindingsResource.js";
import {
  registerWorkbenchContribution,
  WorkbenchPhase,
} from "../common/contributions.js";
import {
  ViewContainerLocation,
  WorkbenchViewContainerId,
  ViewsRegistry,
} from "../common/views.js";
import {
  KeybindingsResourceContribution,
} from "../services/keybinding/browser/keybindingsResourceContribution.js";
import "./parts/dialogs/dialog.contribution.js";
import "./parts/titlebar/titlebarActions.js";
import "./parts/titlebar/menubar.contribution.js";

ViewsRegistry.registerStaticViewContainer({
  id: WorkbenchViewContainerId.Sidebar,
  title: "Navigation",
  location: ViewContainerLocation.Sidebar,
  order: 1,
  isDefault: true,
});
ViewsRegistry.registerStaticViewContainer({
  id: WorkbenchViewContainerId.AuxiliaryBar,
  title: "Auxiliary",
  location: ViewContainerLocation.AuxiliaryBar,
  order: 1,
  isDefault: true,
});

registerWorkbenchContribution(
  "workbench.contrib.keybindingsResource",
  WorkbenchPhase.BlockRestore,
  (accessor) => new KeybindingsResourceContribution({
    service: accessor.get(IKeybindingsResourceService),
  }),
);
