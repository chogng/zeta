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
import {
  registerFilesViews,
} from "../contrib/files/browser/files.contribution.js";
import {
  registerGitViews,
} from "../contrib/scm/browser/scm.contribution.js";
import {
  registerSearchViews,
} from "../contrib/search/browser/search.contribution.js";
import {
  registerChatViews,
} from "../contrib/chat/browser/chat.contribution.js";
import { LxIcon } from "../../base/common/lxicons.js";
import "../contrib/markdown/browser/markdown.contribution.js";
import "../contrib/preferences/browser/preferences.contribution.js";
import "../contrib/quickaccess/browser/commandsQuickAccess.js";
import "./parts/dialogs/dialog.contribution.js";
import "./parts/editor/editorActions.js";
import "./parts/titlebar/menubar.contribution.js";
import "./parts/titlebar/titlebarActions.js";

ViewsRegistry.registerStaticViewContainer({
  id: WorkbenchViewContainerId.Sidebar,
  title: "Explorer",
  location: ViewContainerLocation.Sidebar,
  icon: LxIcon.files,
  order: 1,
  isDefault: true,
});
ViewsRegistry.registerStaticViewContainer({
  id: WorkbenchViewContainerId.Panel,
  title: "Panel",
  location: ViewContainerLocation.Panel,
  order: 1,
  isDefault: true,
});
registerFilesViews();
registerSearchViews();
registerGitViews();
registerChatViews();

registerWorkbenchContribution(
  "workbench.contrib.keybindingsResource",
  WorkbenchPhase.BlockRestore,
  (accessor) => new KeybindingsResourceContribution({
    service: accessor.get(IKeybindingsResourceService),
  }),
);
