import { Keybinding, logicalKey } from "../../../../base/common/keybindings.js";
import { lxiconsLibrary } from "../../../../base/common/lxiconsLibrary.js";
import { Action2, MenuId, registerAction2 } from "../../../../platform/actions/common/actions.js";
import { IConfigurationService } from "../../../../platform/configuration/common/configuration.js";
import { IDialogService } from "../../../../platform/dialogs/common/dialogs.js";
import { ILayoutService } from "../../../../platform/layout/common/layoutService.js";
import type { ServicesAccessor } from "../../../../platform/instantiation/common/instantiation.js";
import { IThemeService } from "../../../../platform/theme/common/themeService.js";
import { registerWorkbenchContribution, WorkbenchPhase } from "../../../common/contributions.js";
import { IUserThemeService } from "../../../common/userThemes.js";
import { ISettingsService } from "../../../services/preferences/common/settings.js";
import { ICodeIndexService } from "../../../../platform/codeIndex/common/codeIndexService.js";
import { IToolSearchService } from "../../../../platform/toolSearch/common/toolSearchService.js";
import { IConnectorService } from "../../../../platform/connectors/common/connectorService.js";
import { IPluginService } from "../../../../platform/plugins/common/pluginService.js";
import { SettingsEditorContribution } from "./settingsEditor.contribution.js";

export const OpenSettingsCommandId = "workbench.action.openSettings";

registerAction2(class OpenSettingsAction extends Action2 {
  constructor() {
    super({
      id: OpenSettingsCommandId,
      title: "Zeta Settings",
      tooltip: "Zeta Settings",
      icon: lxiconsLibrary.gear,
      menu: [
        {
          id: MenuId.TitleBar,
          group: "navigation",
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
    configurationService: accessor.get(IConfigurationService),
    container: accessor.get(ILayoutService).mainContainer,
    dialogService: accessor.get(IDialogService),
    settingsService: accessor.get(ISettingsService),
    themeService: accessor.get(IThemeService),
    userThemeService: accessor.get(IUserThemeService),
    codeIndexService: accessor.get(ICodeIndexService),
    connectorService: accessor.get(IConnectorService),
    pluginService: accessor.get(IPluginService),
    toolSearchService: accessor.get(IToolSearchService),
  }),
);
