import type { IContextViewProvider } from "../../../../base/browser/ui/contextview/contextview.js";
import { noEvent } from "../../../../base/common/event.js";
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
import type { ModelCatalogSource } from "./modelCatalogItems.js";
import type { IContextMenuProvider } from "../../../../base/browser/contextmenu.js";
import type { IClipboardService } from "../../../../platform/clipboard/common/clipboardService.js";
import type { IAccountService } from "../../../../platform/accounts/common/accountService.js";
import { ConnectorCatalogItem } from "./connectorCatalogItem.js";
import { EditorConfigurationContribution } from "./editorConfigurationLayout.js";
import { CoreConfigurationContribution } from "./coreConfigurationLayout.js";
import { SearchIndexConfigurationItems } from "./searchIndexConfigurationItems.js";
import { LocalizationSelectorItem } from "../../localization/browser/localizationSelectorItem.js";
import { MarketplaceCatalogItem } from "./marketplaceCatalogItem.js";
import { ModelCatalogContribution } from "./modelCatalogItems.js";
import { PluginCatalogContribution } from "./pluginCatalogItems.js";
import { CapabilityOverviewContribution, hasSectionOverviewSettings } from "./capabilityOverviewItems.js";
import { ThemePreferenceItem } from "./themePreferenceItem.js";
import { SettingsContributionRegistry } from "./settingsContributions.js";
import { settingsResourceItemId, SettingsSections } from "./settingsLayout.js";
import { WorkspaceTrustItems } from "../../workspace/browser/workspaceTrustItems.js";

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
	readonly modelCatalog?: ModelCatalogSource;
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
			contributions: createSettingsContributionRegistry(options),
			localizationService,
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

function createSettingsContributionRegistry(options: SettingsEditorContributionOptions): SettingsContributionRegistry {
	const clipboardService = options.clipboardService ?? unavailableClipboardService;
	const contextMenuProvider = options.contextMenuProvider ?? unavailableContextMenuProvider;
	const localizationService = options.localizationService ?? unavailableLocalizationService;
	const marketplaceService = options.marketplaceService ?? unavailableMarketplaceService;
	const registry = new SettingsContributionRegistry();
	registry.register(new CoreConfigurationContribution(options.container.ownerDocument, {
		clipboardService,
		configurationService: options.configurationService,
		contextMenuProvider,
		contextViewProvider: options.contextViewProvider,
		workbenchModeService: options.workbenchModeService ?? unavailableWorkbenchModeService,
		preferencesService: options.preferencesService ?? unavailablePreferencesService,
		onStatus: registry.reportStatus,
	}));
	registry.registerLayout('appearance', [{
		id: 'theme',
		title: 'Color theme',
		description: 'Choose and customize the colors used by Zeta.',
		settings: [{
			id: settingsResourceItemId('appearance', 'theme-preference'),
			title: 'Color theme',
			description: 'Choose an appearance or keep Zeta synchronized with your operating system.',
			createView: document => new ThemePreferenceItem(document, {
				clipboardService,
				configurationService: options.configurationService,
				contextMenuProvider,
				dialogService: options.dialogService,
				themeService: options.themeService,
				userThemeService: options.userThemeService,
			}),
		}],
	}]);
	registry.register(new EditorConfigurationContribution(options.container.ownerDocument, {
		clipboardService,
		configurationService: options.configurationService,
		contextMenuProvider,
		contextViewProvider: options.contextViewProvider,
		onStatus: registry.reportStatus,
	}));
	registry.registerLayout('connectors', [{
		id: 'available',
		title: 'Available connectors',
		description: 'Connect external accounts contributed by active plugins.',
		settings: [{
			id: settingsResourceItemId('connectors', 'catalog'),
			title: 'Connector catalog',
			description: 'Manage connector credentials and authorization.',
			createView: document => new ConnectorCatalogItem(document, options.connectorService ?? unavailableConnectorService),
		}],
	}]);
	registry.register(new PluginCatalogContribution(options.pluginService ?? unavailablePluginService, registry.reportStatus));
	registry.registerLayout('languages', [{
		id: 'marketplace',
		title: 'Language packages',
		description: 'Discover and manage Marketplace language extensions.',
		settings: [{
			id: settingsResourceItemId('languages', 'marketplace'),
			title: 'Language Marketplace',
			description: 'Search available language packages.',
			createView: document => new MarketplaceCatalogItem(document, marketplaceService, 'language', localizationService),
		}],
	}]);
	registry.registerLayout('localization', [{
		id: 'display-language',
		title: 'Display language',
		description: 'Choose and install interface languages.',
		settings: [{
			id: settingsResourceItemId('localization', 'interface-language'),
			title: 'Interface language',
			description: 'Choose the language used by the Zeta interface.',
			createView: document => new LocalizationSelectorItem(
				document,
				localizationService,
				options.localeService ?? unavailableLocaleService,
				options.languagePackService ?? unavailableLanguagePackService,
				contextMenuProvider,
				clipboardService,
			),
		}],
	}]);
	registry.registerLayout('marketplace', [{
		id: 'packages',
		title: 'Packages',
		description: 'Discover and manage packages from the signed catalog.',
		settings: [{
			id: settingsResourceItemId('marketplace', 'packages'),
			title: 'Marketplace packages',
			description: 'Search Plugins, Skills, MCP servers, languages, and themes.',
			createView: document => new MarketplaceCatalogItem(document, marketplaceService, undefined, localizationService),
		}],
	}]);
	registry.register(new ModelCatalogContribution({
		clipboardService,
		contextMenuProvider,
		models: options.modelCatalog ?? unavailableModelCatalog,
		accounts: options.accountService ?? unavailableAccountService,
		onStatus: registry.reportStatus,
	}));
	registry.registerLayout('indexing', [{
		id: 'search',
		title: 'Search indexes',
		description: 'Configure tool discovery and semantic workspace search.',
		settings: [{
			id: settingsResourceItemId('indexing', 'configuration'),
			title: 'Indexing configuration',
			description: 'Configure Tool Search and semantic code indexing.',
			createView: document => {
				const item = new SearchIndexConfigurationItems(document, {
					clipboardService,
					codeIndexService: options.codeIndexService ?? unavailableCodeIndexService,
					contextMenuProvider,
					dialogService: options.dialogService,
					toolSearchService: options.toolSearchService ?? unavailableToolSearchService,
				});
				item.activate();
				return item;
			},
		}],
	}]);
	registry.registerLayout('workspace-trust', [{
		id: 'trusted-folders',
		title: 'Trusted folders',
		description: 'Review and revoke folders allowed to run workspace capabilities.',
		settings: [{
			id: settingsResourceItemId('workspace-trust', 'folders'),
			title: 'Workspace trust',
			description: 'Manage the current workspace and durable folder decisions.',
			createView: document => new WorkspaceTrustItems(
				document,
				options.workspaceTrustService ?? unavailableWorkspaceTrustService,
				options.workspaceOpenService ?? unavailableWorkspaceOpenService,
				options.dialogService,
				options.workspaceContextService,
			),
		}],
	}]);
	for (const section of SettingsSections) {
		if (!hasSectionOverviewSettings(section.id)) continue;
		registry.register(new CapabilityOverviewContribution(section.id, options.settingsService));
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

const unavailableModelCatalog: ModelCatalogSource = {
	onDidChangeModels: noEvent,
	listModelCatalog: () => Promise.reject(new Error("Model settings are unavailable.")),
	refreshModels: () => Promise.reject(new Error("Model settings are unavailable.")),
	isModelVisible: () => true,
	setModelVisible: () => Promise.reject(new Error("Model settings are unavailable.")),
};

const unavailableAccountService: IAccountService = {
	onDidChangeAccounts: noEvent,
	onDidCompleteLogin: noEvent,
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
	onDidChange: noEvent,
	list: () => Promise.reject(new Error("Connectors are unavailable.")),
	connectApiToken: () => Promise.reject(new Error("Connectors are unavailable.")),
	connectOAuth: () => Promise.reject(new Error("Connectors are unavailable.")),
	disconnect: () => Promise.reject(new Error("Connectors are unavailable.")),
	refreshOAuth: () => Promise.reject(new Error("Connectors are unavailable.")),
	revokeOAuth: () => Promise.reject(new Error("Connectors are unavailable.")),
};

const unavailablePluginService: IPluginService = {
	onDidChange: noEvent,
	list: () => Promise.reject(new Error("Plugins are unavailable.")),
	enable: () => Promise.reject(new Error("Plugins are unavailable.")),
	disable: () => Promise.reject(new Error("Plugins are unavailable.")),
	grant: () => Promise.reject(new Error("Plugins are unavailable.")),
	revokeGrant: () => Promise.reject(new Error("Plugins are unavailable.")),
	uninstall: () => Promise.reject(new Error("Plugins are unavailable.")),
};

const unavailableMarketplaceService: IMarketplaceService = {
	onDidChangeInstalled: noEvent,
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
	onDidChange: noEvent,
	whenReady: Promise.resolve(),
	translate: (_bundle, _key, fallback) => fallback,
};

const unavailableLocaleService: ILocaleService = {
	locale: "en",
	onDidChangeLocale: noEvent,
	whenReady: Promise.resolve(),
	setLocale: () => Promise.reject(new Error("Locale selection is unavailable.")),
};

const unavailableLanguagePackService: ILanguagePackService = {
	onDidChange: noEvent,
	whenReady: Promise.resolve(),
	catalogs: [],
	availableLocales: [],
	installedPackages: [],
	search: () => Promise.reject(new Error("Language packs are unavailable.")),
	install: () => Promise.reject(new Error("Language packs are unavailable.")),
	refresh: () => Promise.reject(new Error("Language packs are unavailable.")),
};
