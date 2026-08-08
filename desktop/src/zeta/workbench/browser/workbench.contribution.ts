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
import { registerPanelPlaceholderViews } from "../contrib/panel/browser/panel.contribution.js";
import { registerTerminalView } from "../contrib/terminal/browser/terminal.contribution.js";
import { lxiconsLibrary } from "../../base/common/lxiconsLibrary.js";
import "../contrib/markdown/browser/markdown.contribution.js";
import "../contrib/pdf/browser/pdf.contribution.js";
import "../contrib/preferences/browser/preferences.contribution.js";
import "../contrib/quickaccess/browser/commandsQuickAccess.js";
import "../contrib/sash/browser/sash.contribution.js";
import "./parts/dialogs/dialog.contribution.js";
import "./parts/editor/editorActions.js";
import "./parts/titlebar/menubar.contribution.js";
import "./parts/titlebar/titlebarActions.js";

ViewsRegistry.registerStaticViewContainer({
  id: WorkbenchViewContainerId.Sidebar,
  title: "Explorer",
  location: ViewContainerLocation.Sidebar,
  icon: lxiconsLibrary.files,
  order: 1,
  isDefault: true,
});
registerFilesViews();
registerSearchViews();
registerGitViews();
registerChatViews();
registerPanelPlaceholderViews();
registerTerminalView();

registerWorkbenchContribution(
  "workbench.contrib.keybindingsResource",
  WorkbenchPhase.BlockRestore,
  (accessor) => new KeybindingsResourceContribution({
    service: accessor.get(IKeybindingsResourceService),
  }),
);
