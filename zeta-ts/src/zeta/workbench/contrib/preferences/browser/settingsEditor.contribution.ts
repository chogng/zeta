import type { IContextViewProvider } from "../../../../base/browser/ui/contextview/contextview.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import type { IConfigurationService } from "../../../../platform/configuration/common/configurationService.js";
import type { IDialogService } from "../../../../platform/dialogs/common/dialogs.js";
import type { IThemeService } from "../../../../platform/theme/common/themeService.js";
import { ModalEditorPart } from "../../../browser/parts/editor/modalEditorPart.js";
import type { IUserThemeService } from "../../../common/userThemes.js";
import type { ISettingsService } from "../../../services/preferences/common/settings.js";
import type { IPreferencesService } from "../../../services/preferences/common/preferences.js";
import type { ICodeIndexService } from "../../../../platform/codeIndex/common/codeIndexService.js";
import type { IToolSearchService } from "../../../../platform/toolSearch/common/toolSearchService.js";
import type { IConnectorService } from "../../../../platform/connectors/common/connectorService.js";
import type { IPluginService } from "../../../../platform/plugins/common/pluginService.js";
import type { IMarketplaceService } from "../../../../platform/marketplace/common/marketplaceService.js";
import type { ILanguagePackService } from "../../../../platform/languagePacks/common/languagePacksService.js";
import type { ILocaleService } from "../../../services/localization/common/locale.js";
import type { ILocalizationService } from "../../../services/localization/common/localizationService.js";
import type { IWorkspaceContextService } from "../../../../platform/workspace/common/workspace.js";
import type { IWorkspaceTrustService } from "../../../../platform/workspaceTrust/common/workspaceTrustService.js";
import type { IWorkspaceOpenService } from "../../../services/workspaces/browser/workspaceOpenService.js";
import { WorkbenchModeRegistry } from "../../../../product/common/workbenchMode.js";
import { SettingsEditor } from "./settingsEditor.js";
import type { IWorkbenchModeService } from "../../../services/workbenchMode/common/workbenchModeService.js";
import type { ModelSettingsCatalog } from "./modelSettings.js";
import type { IContextMenuProvider } from "../../../../base/browser/contextmenu.js";
import type { IClipboardService } from "../../../../platform/clipboard/common/clipboardService.js";
import type { IAccountService } from "../../../../platform/accounts/common/accountService.js";
import { ConnectorSettingsPane } from "./connectorSettings.js";
import { EditorSettingsPane } from "./editorSettings.js";
import { GeneralSettingsPane } from "./generalSettings.js";
import { IndexingSettingsPane } from "./indexingSettings.js";
import { LocalizationSettingsPane } from "../../localization/browser/localizationSettings.js";
import { MarketplaceSettingsPane } from "./marketplaceSettings.js";
import { ModelSettingsPane } from "./modelSettings.js";
import { PluginSettingsPane } from "./pluginSettings.js";
import { hasSectionOverviewSettings, SectionOverviewSettingsPane } from "./sectionOverviewSettings.js";
import { AppearanceSettingsPane } from "./appearanceSettings.js";
import { SettingsPaneRegistry } from "./settingsPaneRegistry.js";
import { SettingsSections } from "../common/settingsSections.js";
import { WorkspaceTrustEditor } from "../../workspace/browser/workspaceTrustEditor.js";

export interface SettingsEditorContributionOptions {
	readonly clipboardService?: IClipboardService;
	readonly configurationService: IConfigurationService;
	readonly container: HTMLElement;
	readonly contextMenuProvider?: IContextMenuProvider;
	readonly contextViewProvider: IContextViewProvider;
	readonly dialogService: IDialogService;
	readonly settingsService: ISettingsService;
	readonly preferencesService?: IPreferencesService;
	readonly themeService: IThemeService;
	readonly userThemeService: IUserThemeService;
	readonly codeIndexService?: ICodeIndexService;
	readonly connectorService?: IConnectorService;
	readonly pluginService?: IPluginService;
	readonly marketplaceService?: IMarketplaceService;
	readonly languagePackService?: ILanguagePackService;
	readonly localeService?: ILocaleService;
	readonly localizationService?: ILocalizationService;
	readonly toolSearchService?: IToolSearchService;
	readonly workspaceTrustService?: IWorkspaceTrustService;
	readonly workspaceOpenService?: IWorkspaceOpenService;
	readonly workspaceContextService?: IWorkspaceContextService;
	readonly workbenchModeService?: IWorkbenchModeService;
	readonly modelCatalog?: ModelSettingsCatalog;
	readonly accountService?: IAccountService;
}

/** Connects window Settings state to its modal editor host and content. */
export class SettingsEditorContribution extends DisposableOwner {
	private readonly editor: SettingsEditor;
	private readonly modalEditor: ModalEditorPart;

	constructor(options: SettingsEditorContributionOptions) {
		super();
		const localizationService = options.localizationService ?? unavailableLocalizationService;
		this.editor = this.own(new SettingsEditor(options.container, {
			localizationService,
			paneRegistry: createSettingsPaneRegistry(options),
			settingsService: options.settingsService,
		}));
		this.modalEditor = this.own(new ModalEditorPart({
			container: options.container,
			title: localizationService.translate("zeta.settings", "chrome.modalTitle", "Zeta Settings"),
			content: this.editor.element,
			focusContent: () => this.editor.focus(),
		}));
		this.modalEditor.element.classList.add("zeta-settings-modal");

		this.own(this.modalEditor.onDidRequestClose(() => {
			options.settingsService.close();
		}));
		this.own(options.settingsService.onDidChangeVisibility((visible) => {
			if (visible) this.show();
			else {
				this.editor.cancelPendingChanges();
				this.modalEditor.hide();
			}
		}));
		if (options.settingsService.isOpen) this.show();
	}

	private show(): void {
		this.modalEditor.show();
		this.editor.layout();
	}
}

function createSettingsPaneRegistry(options: SettingsEditorContributionOptions): SettingsPaneRegistry {
	const clipboardService = options.clipboardService ?? unavailableClipboardService;
	const contextMenuProvider = options.contextMenuProvider ?? unavailableContextMenuProvider;
	const localizationService = options.localizationService ?? unavailableLocalizationService;
	const marketplaceService = options.marketplaceService ?? unavailableMarketplaceService;
	const registry = new SettingsPaneRegistry();
	registry.register("general", {
		create: container => new GeneralSettingsPane(container, {
			clipboardService,
			configurationService: options.configurationService,
			contextMenuProvider,
			contextViewProvider: options.contextViewProvider,
			workbenchModeService: options.workbenchModeService ?? unavailableWorkbenchModeService,
			preferencesService: options.preferencesService ?? unavailablePreferencesService,
		}),
	});
	registry.register("appearance", {
		create: container => new AppearanceSettingsPane(container, {
			clipboardService,
			configurationService: options.configurationService,
			contextMenuProvider,
			dialogService: options.dialogService,
			themeService: options.themeService,
			userThemeService: options.userThemeService,
		}),
	});
	registry.register("editor", {
		create: container => new EditorSettingsPane(container, {
			clipboardService,
			configurationService: options.configurationService,
			contextMenuProvider,
			contextViewProvider: options.contextViewProvider,
		}),
	});
	registry.register("connectors", {
		create: container => new ConnectorSettingsPane(container, options.connectorService ?? unavailableConnectorService),
	});
	registry.register("plugins", {
		create: container => new PluginSettingsPane(container, options.pluginService ?? unavailablePluginService),
	});
	registry.register("languages", {
		create: container => new MarketplaceSettingsPane(container, marketplaceService, "language", localizationService),
	});
	registry.register("localization", {
		create: container => new LocalizationSettingsPane(
			container,
			localizationService,
			options.localeService ?? unavailableLocaleService,
			options.languagePackService ?? unavailableLanguagePackService,
			contextMenuProvider,
			clipboardService,
		),
	});
	registry.register("marketplace", {
		create: container => new MarketplaceSettingsPane(container, marketplaceService, undefined, localizationService),
	});
	registry.register("models", {
		create: container => new ModelSettingsPane(container, {
			clipboardService,
			contextMenuProvider,
			models: options.modelCatalog ?? unavailableModelCatalog,
			accounts: options.accountService ?? unavailableAccountService,
		}),
	});
	registry.register("indexing", {
		create: container => new IndexingSettingsPane(container, {
			clipboardService,
			codeIndexService: options.codeIndexService ?? unavailableCodeIndexService,
			contextMenuProvider,
			dialogService: options.dialogService,
			toolSearchService: options.toolSearchService ?? unavailableToolSearchService,
		}),
	});
	registry.register("workspace-trust", {
		create: container => new WorkspaceTrustEditor(
			container,
			options.workspaceTrustService ?? unavailableWorkspaceTrustService,
			options.workspaceOpenService ?? unavailableWorkspaceOpenService,
			options.dialogService,
			options.workspaceContextService,
		),
	});
	for (const section of SettingsSections) {
		if (!hasSectionOverviewSettings(section.id)) continue;
		registry.register(section.id, {
			create: container => new SectionOverviewSettingsPane(container, section.id, options.settingsService),
		});
	}
	return registry;
}

const unavailableClipboardService: IClipboardService = {
	writeText: () => Promise.reject(new Error("Clipboard access is unavailable.")),
};

const unavailablePreferencesService: IPreferencesService = {
	openSettings: () => undefined,
	openKeybindings: () => Promise.reject(new Error('Keyboard Shortcuts editor is unavailable.')),
};

const unavailableContextMenuProvider: IContextMenuProvider = {
	showContextMenu: options => options.onHide?.(true),
};

const unavailableCodeIndexService: ICodeIndexService = {
	readConfig: () => Promise.reject(new Error("Code index settings are unavailable.")),
	configureProvider: () => Promise.reject(new Error("Code index settings are unavailable.")),
	configure: () => Promise.reject(new Error("Code index settings are unavailable.")),
	authorize: () => Promise.reject(new Error("Code index settings are unavailable.")),
	revoke: () => Promise.reject(new Error("Code index settings are unavailable.")),
	status: () => Promise.reject(new Error("Code index settings are unavailable.")),
	cancel: () => Promise.reject(new Error("Code index settings are unavailable.")),
	retry: () => Promise.reject(new Error("Code index settings are unavailable.")),
};

const unavailableWorkbenchModeService: IWorkbenchModeService = {
	currentModeId: WorkbenchModeRegistry.defaultModeId,
	availableModes: WorkbenchModeRegistry.definitions.map(({ id, label }) => ({ id, label })),
	switchMode: () => Promise.reject(new Error("Workbench mode switching is unavailable.")),
	resetMode: () => Promise.reject(new Error("Workbench mode switching is unavailable.")),
};

const unavailableModelCatalog: ModelSettingsCatalog = {
	onDidChangeModels: () => ({ dispose() {}, [Symbol.dispose]() {} }),
	listModelCatalog: () => Promise.reject(new Error("Model settings are unavailable.")),
	refreshModels: () => Promise.reject(new Error("Model settings are unavailable.")),
	isModelVisible: () => true,
	setModelVisible: () => Promise.reject(new Error("Model settings are unavailable.")),
};

const unavailableAccountService: IAccountService = {
	onDidChangeAccounts: () => ({ dispose() {}, [Symbol.dispose]() {} }),
	onDidCompleteLogin: () => ({ dispose() {}, [Symbol.dispose]() {} }),
	read: () => Promise.reject(new Error("Account settings are unavailable.")),
	startLogin: () => Promise.reject(new Error("Account sign-in is unavailable.")),
	cancelLogin: () => Promise.reject(new Error("Kimi sign-in is unavailable.")),
	logout: () => Promise.reject(new Error("Account settings are unavailable.")),
};

const unavailableToolSearchService: IToolSearchService = {
	readConfig: () => Promise.reject(new Error("Tool Search settings are unavailable.")),
	configure: () => Promise.reject(new Error("Tool Search settings are unavailable.")),
};

const unavailableWorkspaceTrustService: IWorkspaceTrustService = {
	list: () => Promise.reject(new Error("Workspace Trust settings are unavailable.")),
	read: () => Promise.reject(new Error("Workspace Trust settings are unavailable.")),
	set: () => Promise.reject(new Error("Workspace Trust settings are unavailable.")),
	forget: () => Promise.reject(new Error("Workspace Trust settings are unavailable.")),
};

const unavailableWorkspaceOpenService: IWorkspaceOpenService = {
	canOpenFolder: false,
	canOpenWorkspace: false,
	openFolder: () => Promise.reject(new Error("Folder picking is unavailable.")),
	openWorkspace: () => Promise.reject(new Error("Workspace opening is unavailable.")),
	pickFolder: () => Promise.reject(new Error("Folder picking is unavailable.")),
};

const unavailableConnectorService: IConnectorService = {
	onDidChange: () => ({ dispose() {}, [Symbol.dispose]() {} }),
	list: () => Promise.reject(new Error("Connectors are unavailable.")),
	connectApiToken: () => Promise.reject(new Error("Connectors are unavailable.")),
	connectOAuth: () => Promise.reject(new Error("Connectors are unavailable.")),
	disconnect: () => Promise.reject(new Error("Connectors are unavailable.")),
	refreshOAuth: () => Promise.reject(new Error("Connectors are unavailable.")),
	revokeOAuth: () => Promise.reject(new Error("Connectors are unavailable.")),
};

const unavailablePluginService: IPluginService = {
	onDidChange: () => ({ dispose() {}, [Symbol.dispose]() {} }),
	list: () => Promise.reject(new Error("Plugins are unavailable.")),
	enable: () => Promise.reject(new Error("Plugins are unavailable.")),
	disable: () => Promise.reject(new Error("Plugins are unavailable.")),
	grant: () => Promise.reject(new Error("Plugins are unavailable.")),
	revokeGrant: () => Promise.reject(new Error("Plugins are unavailable.")),
	uninstall: () => Promise.reject(new Error("Plugins are unavailable.")),
};

const unavailableMarketplaceService: IMarketplaceService = {
	onDidChangeInstalled: () => ({ dispose() {}, [Symbol.dispose]() {} }),
	cachedBrowse: () => undefined,
	browse: () => Promise.reject(new Error("Marketplace is unavailable.")),
	refreshBrowse: () => Promise.reject(new Error("Marketplace is unavailable.")),
	search: () => Promise.reject(new Error("Marketplace is unavailable.")),
	get: () => Promise.reject(new Error("Marketplace is unavailable.")),
	download: () => Promise.reject(new Error("Marketplace is unavailable.")),
	install: () => Promise.reject(new Error("Marketplace is unavailable.")),
	update: () => Promise.reject(new Error("Marketplace is unavailable.")),
	uninstall: () => Promise.reject(new Error("Marketplace is unavailable.")),
	listInstalled: () => Promise.reject(new Error("Marketplace is unavailable.")),
	acquireCapability: () => Promise.reject(new Error("Marketplace is unavailable.")),
	releaseCapability: () => Promise.reject(new Error("Marketplace is unavailable.")),
	openResource: () => Promise.reject(new Error("Marketplace is unavailable.")),
};

const unavailableLocalizationService: ILocalizationService = {
	onDidChange: () => ({ dispose() {}, [Symbol.dispose]() {} }),
	whenReady: Promise.resolve(),
	translate: (_bundle, _key, fallback) => fallback,
};

const unavailableLocaleService: ILocaleService = {
	locale: "en",
	onDidChangeLocale: () => ({ dispose() {}, [Symbol.dispose]() {} }),
	whenReady: Promise.resolve(),
	setLocale: () => Promise.reject(new Error("Locale selection is unavailable.")),
};

const unavailableLanguagePackService: ILanguagePackService = {
	onDidChange: () => ({ dispose() {}, [Symbol.dispose]() {} }),
	whenReady: Promise.resolve(),
	catalogs: [],
	availableLocales: [],
	installedPackages: [],
	search: () => Promise.reject(new Error("Language packs are unavailable.")),
	install: () => Promise.reject(new Error("Language packs are unavailable.")),
	refresh: () => Promise.reject(new Error("Language packs are unavailable.")),
};
