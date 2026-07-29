import { Keybinding, logicalKey } from "../../../../base/common/keybindings.js";
import { LxIcon } from "../../../../base/common/lxicons.js";
import { Action2, MenuId, registerAction2 } from "../../../../platform/actions/common/actions.js";
import type { ServicesAccessor } from "../../../../platform/instantiation/common/instantiation.js";
import { IWorkbenchWindowService } from "../../../browser/window.js";
import { registerWorkbenchContribution, WorkbenchPhase } from "../../../common/contributions.js";
import { ISettingsService } from "../../../services/preferences/common/settings.js";
import { SettingsEditorContribution } from "./settingsEditor.contribution.js";

export const OpenSettingsCommandId = "workbench.action.openSettings";

registerAction2(class OpenSettingsAction extends Action2 {
  constructor() {
    super({
      id: OpenSettingsCommandId,
      title: "Zeta Settings",
      tooltip: "Zeta Settings",
      icon: LxIcon.gear,
      menu: [
        {
          id: MenuId.TitleBar,
          group: "navigation",
          order: 100,
        },
        {
          id: MenuId.ChatTitle,
          group: "settings",
          order: 100,
        },
        {
          id: MenuId.EditorTitle,
          group: "settings",
          order: 100,
        },
      ],
      keybinding: {
        primary: Keybinding.single(logicalKey(",", {
          primaryKey: true,
        })),
      },
      f1: true,
    });
  }

  override run(accessor: ServicesAccessor): void {
    accessor.get(ISettingsService).open();
  }
});

registerWorkbenchContribution(
  "workbench.contrib.settingsEditor",
  WorkbenchPhase.BlockStartup,
  (accessor) => new SettingsEditorContribution({
    container: accessor.get(IWorkbenchWindowService).root,
    settingsService: accessor.get(ISettingsService),
  }),
);
