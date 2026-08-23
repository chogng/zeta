import { Keybinding, logicalKey } from "../../../../base/common/keybindings.js";
import { lxiconsLibrary } from "../../../../base/common/lxiconsLibrary.js";
import { Action2, MenuId, registerAction2 } from "../../../../platform/actions/common/actions.js";
import { IConfigurationService } from "../../../../platform/configuration/common/configurationService.js";
import { IContextViewService } from "../../../../platform/contextview/browser/contextView.js";
import { IDialogService } from "../../../../platform/dialogs/common/dialogs.js";
import { ILayoutService } from "../../../../platform/layout/common/layoutService.js";
import type { ServicesAccessor } from "../../../../platform/instantiation/common/instantiation.js";
import { IThemeService } from "../../../../platform/theme/common/themeService.js";
import { registerWorkbenchContribution, WorkbenchPhase } from "../../../common/contributions.js";
import { IUserThemeService } from "../../../common/userThemes.js";
import { ISettingsService } from "../../../services/preferences/common/settings.js";
import { IPreferencesService } from "../../../services/preferences/common/preferences.js";
import { ICodeIndexService } from "../../../../platform/codeIndex/common/codeIndexService.js";
import { IToolSearchService } from "../../../../platform/toolSearch/common/toolSearchService.js";
import { IConnectorService } from "../../../../platform/connectors/common/connectorService.js";
import { IPluginService } from "../../../../platform/plugins/common/pluginService.js";
import { IMarketplaceService } from "../../../../platform/marketplace/common/marketplaceService.js";
import { ILanguagePackService } from "../../../../platform/languagePacks/common/languagePacksService.js";
import { ILocaleService } from "../../../services/localization/common/locale.js";
import { ILocalizationService } from "../../../services/localization/common/localizationService.js";
import { IWorkspaceContextService } from "../../../../platform/workspace/common/workspace.js";
import { IWorkspaceTrustService } from "../../../../platform/workspaceTrust/common/workspaceTrustService.js";
import { IWorkspaceOpenService } from "../../../services/workspaces/browser/workspaceOpenService.js";
import { SettingsEditorContribution } from "./settingsEditor.contribution.js";
import { IWorkbenchModeService } from "../../../services/workbenchMode/common/workbenchModeService.js";
import { IChatService } from "../../../services/chat/common/chatService.js";
import { IClipboardService } from "../../../../platform/clipboard/common/clipboardService.js";
import { IContextMenuService } from "../../../../platform/contextview/browser/contextMenu.js";
import { IAccountService } from "../../../../platform/accounts/common/accountService.js";
import { OpenKeyboardShortcutsCommandId, OpenSettingsCommandId } from "../common/preferences.js";
import "./keyboardLayoutPicker.js";
import './keyboardShortcutsEditor.contribution.js';

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
		accessor.get(IPreferencesService).openSettings();
	}
});

registerAction2(class OpenKeyboardShortcutsAction extends Action2 {
	constructor() {
		super({
			id: OpenKeyboardShortcutsCommandId,
			title: 'Preferences: Open Keyboard Shortcuts',
			f1: true,
		});
	}

	override run(accessor: ServicesAccessor): Promise<void> {
		return accessor.get(IPreferencesService).openKeybindings();
	}
});

registerWorkbenchContribution(
	"workbench.contrib.settingsEditor",
	WorkbenchPhase.BlockStartup,
	(accessor) => new SettingsEditorContribution({
		configurationService: accessor.get(IConfigurationService),
		clipboardService: accessor.get(IClipboardService),
		container: accessor.get(ILayoutService).mainContainer,
		contextMenuProvider: accessor.get(IContextMenuService),
		contextViewProvider: accessor.get(IContextViewService),
		dialogService: accessor.get(IDialogService),
		settingsService: accessor.get(ISettingsService),
		preferencesService: accessor.get(IPreferencesService),
		themeService: accessor.get(IThemeService),
		userThemeService: accessor.get(IUserThemeService),
		codeIndexService: accessor.get(ICodeIndexService),
		connectorService: accessor.get(IConnectorService),
		pluginService: accessor.get(IPluginService),
		marketplaceService: accessor.get(IMarketplaceService),
		languagePackService: accessor.get(ILanguagePackService),
		localeService: accessor.get(ILocaleService),
		localizationService: accessor.get(ILocalizationService),
		toolSearchService: accessor.get(IToolSearchService),
		workspaceTrustService: accessor.get(IWorkspaceTrustService),
		workspaceOpenService: accessor.get(IWorkspaceOpenService),
		workspaceContextService: accessor.get(IWorkspaceContextService),
		workbenchModeService: accessor.get(IWorkbenchModeService),
		modelCatalog: accessor.get(IChatService),
		accountService: accessor.get(IAccountService),
	}),
);
