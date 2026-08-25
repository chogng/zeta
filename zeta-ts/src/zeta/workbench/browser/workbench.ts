import "./style.js";
import type { FsChanged } from "../../../../generated/app-server/types.js";
import { bindResizableLayout } from "../../base/browser/ui/resizable/resizable.js";
import { disposableWindowTimeout } from "../../base/browser/scheduler.js";
import {
	type IDisposable,
	DisposableOwner,
} from "../../base/common/lifecycle.js";
import { assertDefined } from "../../base/common/types.js";
import { WorkbenchModeRegistry, type WorkbenchModeId } from "../../product/common/workbenchMode.js";
import { URI } from "../../base/common/uri.js";
import { AccessibilityService } from "../../platform/accessibility/browser/accessibilityService.js";
import { IAccessibilityService } from "../../platform/accessibility/common/accessibility.js";
import { ConsoleLogSink } from "../../platform/log/common/consoleLogSink.js";
import { ILogService } from "../../platform/log/common/logService.js";
import { LogService } from "../../platform/log/common/logServiceImpl.js";
import { BrowserLifecycleService } from "../../platform/lifecycle/browser/browserLifecycleService.js";
import { ILifecycleService, type ShutdownReason } from "../../platform/lifecycle/common/lifecycleService.js";
import { IDebugAdapterProcessService } from "../../platform/debug/common/debugAdapterProcessService.js";
import { IExtensionHostApi } from "../../platform/extensionHost/common/extensionHostApi.js";
import type { IRendererHost } from "../../platform/renderer/common/rendererHost.js";
import { IRemoteConnectionService } from "../../platform/remote/common/remoteConnectionService.js";
import { UnavailableRemoteConnectionService } from "../../platform/remote/common/remoteConnectionService.js";
import { IRemoteTunnelService } from "../../platform/remote/common/remoteTunnelService.js";
import { UnavailableRemoteTunnelService } from "../../platform/remote/common/remoteTunnelService.js";
import type {
	INativeHostApi,
} from "../../platform/native/common/nativeHost.js";
import { MenuId } from "../../platform/actions/common/actions.js";
import type { IConfigurationApi } from "../../platform/configuration/common/configurationIpc.js";
import { IConfigurationService } from "../../platform/configuration/common/configurationService.js";
import { IStorageService, WillSaveStateReason } from "../../platform/storage/common/storage.js";
import { BrowserLayoutService } from "../../platform/layout/browser/layoutService.js";
import { ILayoutService } from "../../platform/layout/common/layoutService.js";
import "../../platform/layout/browser/zIndexRegistry.js";
import {
	InstantiationService,
	ServiceCollection,
} from "../../platform/instantiation/common/instantiation.js";
import type { IKeybindingsResourceApi } from "../../platform/keybinding/common/keybindingsResource.js";
import type { IKeyboardLayoutProvider } from "../../platform/keyboardLayout/common/keyboardLayout.js";
import {
	BrowserDialogHandler,
} from "../../platform/dialogs/browser/browserDialogHandler.js";
import {
	IDialogService,
} from "../../platform/dialogs/common/dialogs.js";
import {
	BrowserFileService,
} from "../../platform/files/browser/fileService.js";
import {
	IFileService,
} from "../../platform/files/common/files.js";
import {
	bindColorTheme,
} from "../../platform/theme/browser/themeStyles.js";
import {
	IFileIconThemeService,
} from "../../platform/theme/browser/fileIconThemeService.js";
import {
	SetiFileIconThemeService,
} from "../../platform/theme/browser/setiFileIconTheme.js";
import {
	IThemeService,
	ThemeService,
} from "../../platform/theme/common/themeService.js";
import { type IWorkspace, IWorkspaceContextService, WorkbenchState, workbenchStateFromWorkspace, workspaceOpenTarget } from "../../platform/workspace/common/workspace.js";
import { WorkbenchConfiguration } from "../common/configuration.js";
import {
	type WorkbenchContributionHost,
	WorkbenchContributionsRegistry,
	WorkbenchPhase,
} from "../common/contributions.js";
import {
	IDialogsModel,
	IWorkbenchDialogHandler,
} from "../common/dialogs.js";
import { INativeHostService } from "../common/services.js";
import { resolveWorkbenchColorTheme } from "../common/theme.js";
import { IUserThemeService, type IUserThemeService as IUserThemeServiceContract, UnavailableUserThemeService } from "../common/userThemes.js";
import {
	ViewContainerLocation,
} from "../common/views.js";
import {
	IStatusbarService,
	StatusbarService,
} from "../services/statusbar/browser/statusbar.js";
import {
	WorkspaceContextService,
} from "../services/workspaces/browser/workspaceContextService.js";
import {
	IWorkspaceOpenService,
	WorkspaceOpenService,
} from "../services/workspaces/browser/workspaceOpenService.js";
import { RecentWorkspacesService } from "../services/workspaces/browser/recentWorkspacesService.js";
import { IRecentWorkspacesService } from "../services/workspaces/common/recentWorkspacesService.js";
import {
	IViewDescriptorService,
	ViewDescriptorService,
} from "../services/views/common/viewDescriptorService.js";
import {
	IViewsService,
	ViewsService,
} from "../services/views/browser/viewsService.js";
import { AppServerSessionsManagementService } from "../../sessions/services/sessions/browser/appServerSessionsManagementService.js";
import { ISessionsManagementService } from "../../sessions/services/sessions/common/sessionsManagementService.js";
import {
	WorkbenchConfigurationService,
} from "../services/configuration/browser/configurationService.js";
import type {
	WorkbenchContextMenuServiceFactory,
} from "../services/contextmenu/browser/workbenchContextMenuService.js";
import {
	DialogService,
} from "../services/dialogs/common/dialogService.js";
import {
	bindEditorContextKeys,
	bindWorkbenchActiveCompositeContextKeys,
	bindWorkbenchContextKeys,
	bindWorkbenchPartVisibilityContextKeys,
} from "./contextkeys.js";
import { WorkbenchThemeController } from "./theme.js";
import { WorkbenchLayout, type WorkbenchDefaultLayout } from "./layout.js";
import { IWorkbenchLayoutService, type WorkbenchPartId } from "../services/layout/browser/layoutService.js";
import { BrowserStorageService } from "../services/storage/browser/storageService.js";
import { SystemOutputService } from "../services/output/browser/systemOutputService.js";
import { IWorkspaceSearchService } from "../../platform/search/common/search.js";
import { BrowserWorkspaceSearchService } from "../../platform/search/browser/searchService.js";
import type { WorkbenchPart } from "./part.js";
import { AuxiliarybarPart } from "./parts/auxiliarybar/auxiliarybarPart.js";
import { EditorPart, IEditorPart } from "./parts/editor/editorPart.js";
import { EditorPanes } from './parts/editor/editorRegistry.js';
import { PanelPart } from "./parts/panel/panelPart.js";
import { SidebarPart } from "./parts/sidebar/sidebarPart.js";
import { StatusbarPart } from "./parts/statusbar/statusbarPart.js";
import type {
	TitlebarPartFactory,
} from "./parts/titlebar/titlebarPart.js";
import { PaneComposite } from "./parts/views/paneComposite.js";
import { WorkbenchWindow } from "./window.js";
import { TerminalService } from "../services/terminal/browser/terminalService.js";
import { ITerminalService } from "../services/terminal/common/terminal.js";
import { ITextFileService, TextFileService } from "../services/textfile/common/textFileService.js";
import { ITextMateService } from "../services/textMate/common/textMateService.js";
import { BrowserTextMateService } from "../services/textMate/browser/browserTextMateService.js";
import { AppServerExtensionService } from "../services/extensions/browser/appServerExtensionService.js";
import { IExtensionService } from "../services/extensions/common/extensionService.js";
import { AppServerRemoteAgentService } from "../services/remote/browser/appServerRemoteAgentService.js";
import { IRemoteAgentService } from "../services/remote/common/remoteAgentService.js";
import { ILanguageFeaturesService, LanguageFeaturesService } from "../services/language/common/languageFeaturesService.js";
import { GitService } from "../services/git/browser/gitService.js";
import { IGitService } from "../services/git/common/gitService.js";
import { ChatService } from "../services/chat/browser/chatService.js";
import { IChatService } from "../services/chat/common/chatService.js";
import { ChatContextPickService } from "../services/chat/browser/chatContextPickService.js";
import { IChatContextPickService } from "../services/chat/common/chatContextService.js";
import { ICodeIndexService } from "../../platform/codeIndex/common/codeIndexService.js";
import { AppServerCodeIndexService } from "../services/codeIndex/browser/appServerCodeIndexService.js";
import { IToolSearchService } from "../../platform/toolSearch/common/toolSearchService.js";
import { IWorkspaceTrustService } from "../../platform/workspaceTrust/common/workspaceTrustService.js";
import { IKeybindingsResourceService } from "../../platform/keybinding/common/keybindingsResource.js";
import { IKeyboardLayoutService } from "../../platform/keyboardLayout/common/keyboardLayout.js";
import { AppServerWorkspaceTrustService } from "../services/workspaces/browser/appServerWorkspaceTrustService.js";
import { IConnectorService } from "../../platform/connectors/common/connectorService.js";
import { AppServerConnectorService } from "../services/connectors/browser/appServerConnectorService.js";
import { IAccountService } from "../../platform/accounts/common/accountService.js";
import { AppServerAccountService } from "../services/accounts/browser/appServerAccountService.js";
import { IPluginService } from "../../platform/plugins/common/pluginService.js";
import { AppServerPluginService } from "../services/plugins/browser/appServerPluginService.js";
import { IMarketplaceService } from "../../platform/marketplace/common/marketplaceService.js";
import { AppServerMarketplaceService } from "../services/marketplace/browser/appServerMarketplaceService.js";
import { MarketplaceLanguagePackService } from "../../platform/languagePacks/browser/marketplaceLanguagePackService.js";
import { ILanguagePackService } from "../../platform/languagePacks/common/languagePacksService.js";
import { ILocaleService, WorkbenchLocaleService } from "../services/localization/common/locale.js";
import { ILocalizationService } from "../services/localization/common/localizationService.js";
import { WorkbenchLocalizationService } from "../services/localization/browser/workbenchLocalizationService.js";
import { builtinLanguagePackCatalogs } from "../services/localization/common/localizationCatalogs.js";
import { AppServerToolSearchService } from "../services/toolSearch/browser/appServerToolSearchService.js";
import { ISymbolIndexApi } from "../../platform/symbolIndex/common/symbolIndexApi.js";
import { AccessibleViewInformationService, IAccessibleViewInformationService } from "../services/accessibility/common/accessibleViewInformationService.js";
import { NativeAccessibilityService } from "../services/accessibility/electron-browser/accessibilityService.js";
import { BrowserUntitledTextEditorService } from "../services/untitled/browser/browserUntitledTextEditorService.js";
import { IUntitledTextEditorService } from "../services/untitled/common/untitledTextEditorService.js";
import { BrowserWorkingCopyService } from "../services/workingCopy/browser/browserWorkingCopyService.js";
import { IWorkingCopyService } from "../services/workingCopy/common/workingCopyService.js";
import { IndexedDbWorkingCopyBackupService } from "../services/workingCopy/browser/indexedDbWorkingCopyBackupService.js";
import { WorkingCopyBackupTracker } from "../services/workingCopy/browser/workingCopyBackupTracker.js";
import { IWorkingCopyBackupService, type WorkingCopyBackup } from "../services/workingCopy/common/workingCopyBackupService.js";
import { projectExtensionTokenTheme } from "../services/textMate/common/textMateThemeProjection.js";
import { BrowserWorkspaceEditService } from "../services/language/browser/browserWorkspaceEditService.js";
import { IWorkspaceEditService } from "../services/language/common/workspaceEditService.js";
import { ITextModelService } from "../../editor/common/services/textModelService.js";
import { BrowserBulkEditService } from "../contrib/bulkEdit/browser/bulkEditService.js";
import { IBulkEditService } from "../contrib/bulkEdit/common/bulkEdit.js";
import { getBrowserTextModelService } from "../../editor/browser/services/browserTextModelService.js";
import { getBrowserTextResourceStore } from "../contrib/codeEditor/browser/browserTextResourceStore.js";
import { AppServerLanguageProviders } from "../services/language/browser/appServerLanguageProviders.js";
import { AppServerLanguageDiagnosticsService } from "../services/language/browser/appServerLanguageDiagnosticsService.js";
import { AppServerCodeIntelligenceDocumentService } from "../services/codeIntelligence/browser/appServerCodeIntelligenceDocumentService.js";
import { ICodeIntelligenceDocumentService } from "../services/codeIntelligence/common/codeIntelligenceDocumentService.js";
import { AppServerLanguageServerStatusService } from "../services/language/browser/appServerLanguageServerStatusService.js";
import { ILanguageServerStatusService } from "../services/language/common/languageServerStatusService.js";
import { ILanguageDiagnosticsService } from "../services/language/common/languageDiagnosticsService.js";
import { OutputService } from "../services/output/browser/outputService.js";
import { IOutputService } from "../services/output/common/outputService.js";
import { IWorkbenchHostService } from "../services/host/common/workbenchHostService.js";
import { BrowserEditorService } from "../services/editor/browser/browserEditorService.js";
import { IEditorService } from "../services/editor/common/editorService.js";
import { OUTPUT_VIEW_ID } from "../contrib/output/common/output.js";
import { createEditorLineGutterDecorations } from "./parts/editor/editorGutterDecorations.js";
import { createEditorDecorationSources } from "./parts/editor/editorDecorations.js";
import { installWorkbenchServiceContributions } from "./workbenchServiceContributions.js";
import { WorkbenchInteractionServices } from "./workbenchInteractionServices.js";
import { ConnectToRemoteCommandId } from "../contrib/remote/browser/remoteActions.js";
import type { IUserKeyboardLayoutApi } from "../../platform/keyboardLayout/common/userKeyboardLayout.js";
import { WorkbenchModeService } from "../services/workbenchMode/browser/workbenchModeService.js";
import { IWorkbenchModeService } from "../services/workbenchMode/common/workbenchModeService.js";
import { BrowserClipboardService } from "../../platform/clipboard/browser/browserClipboardService.js";
import { IClipboardService } from "../../platform/clipboard/common/clipboardService.js";

const DEFAULT_WORKBENCH_LAYOUT = {
	parts: {
		sidebar: false,
		auxiliarybar: false,
		agentSidebar: false,
		panel: false,
	},
} as const satisfies WorkbenchDefaultLayout;

/** Host-specific inputs required to construct a workbench. */
export interface IStartWorkbenchOptions {
	readonly modeId: WorkbenchModeId;
	readonly defaultLayout?: WorkbenchDefaultLayout;
	readonly api: IRendererHost;
	readonly container: HTMLElement;
	readonly workspace: IWorkspace;
	readonly configurationApi?: IConfigurationApi;
	readonly keybindingsResourceApi?: IKeybindingsResourceApi;
	readonly keyboardLayoutProvider?: IKeyboardLayoutProvider;
	readonly userKeyboardLayoutApi?: IUserKeyboardLayoutApi;
	readonly nativeHostApi?: INativeHostApi;
	readonly userThemeService?: IUserThemeServiceContract;
	readonly createContextMenuService: WorkbenchContextMenuServiceFactory;
	readonly createTitlebarPart: TitlebarPartFactory;
	readonly switchWorkbenchMode: (modeId: WorkbenchModeId) => Promise<void>;
}

/** Starts the browser workbench and binds its commands to the initial UI. */
export function startWorkbench({
	modeId,
	defaultLayout,
	api,
	container,
	workspace,
	configurationApi,
	keybindingsResourceApi,
	keyboardLayoutProvider,
	userKeyboardLayoutApi,
	nativeHostApi,
	userThemeService,
	createContextMenuService,
	createTitlebarPart,
	switchWorkbenchMode,
}: IStartWorkbenchOptions): Workbench {
	return new Workbench(
		modeId,
		defaultLayout,
		api,
		container,
		workspace,
		configurationApi,
		keybindingsResourceApi,
		keyboardLayoutProvider,
		userKeyboardLayoutApi,
		nativeHostApi,
		userThemeService,
		createContextMenuService,
		createTitlebarPart,
		switchWorkbenchMode,
	);
}

/** Owns the renderer workbench, its parts, commands, and runtime layout. */
export class Workbench extends DisposableOwner {
	/** Resolves after dirty working copies are restored and AfterRestored contributions are active. */
	readonly whenRestored: Promise<void>;
	private readonly workspaceContext: WorkspaceContextService;
	private readonly storage: BrowserStorageService;
	private readonly editor: EditorPart;
	private readonly workbenchLayout: WorkbenchLayout;
	private readonly workingCopyBackups: IndexedDbWorkingCopyBackupService;
	private readonly workingCopyBackupTracker: WorkingCopyBackupTracker;
	private readonly workbenchWindow: WorkbenchWindow;
	private readonly logService: ILogService;
	private readonly lifecycleService: ILifecycleService;
	private readonly ownerWindow: Window;
	private restoreActiveViewContainers: (() => void) | undefined;
	private workspaceSwitchQueue: Promise<void> = Promise.resolve();

	constructor(
		modeId: WorkbenchModeId,
		defaultLayout: WorkbenchDefaultLayout | undefined,
		api: IRendererHost,
		workbenchRoot: HTMLElement,
		workspace: IWorkspace,
		configurationApi: IConfigurationApi | undefined,
		keybindingsResourceApi: IKeybindingsResourceApi | undefined,
		keyboardLayoutProvider: IKeyboardLayoutProvider | undefined,
		userKeyboardLayoutApi: IUserKeyboardLayoutApi | undefined,
		nativeHostApi: INativeHostApi | undefined,
		userThemeService: IUserThemeServiceContract | undefined,
		createContextMenuService: WorkbenchContextMenuServiceFactory,
		createTitlebarPart: TitlebarPartFactory,
		switchWorkbenchMode: (modeId: WorkbenchModeId) => Promise<void>,
	) {
		super();
		const mode = WorkbenchModeRegistry.get(modeId);
		const services = new ServiceCollection();
		const instantiationService = new InstantiationService(services);
		const logService = this.own(new LogService({ sinks: [new ConsoleLogSink()] }));
		this.logService = logService;
		services.set(ILogService, logService);
		services.set(IExtensionHostApi, api.extensionHost);
		services.set(ISymbolIndexApi, api.symbolIndex);
		if (api.debugAdapter) services.set(IDebugAdapterProcessService, api.debugAdapter);
		const remoteAgentService = this.own(new AppServerRemoteAgentService({ api: api.appServer, remoteApi: api.remote }));
		services.set(IRemoteAgentService, remoteAgentService);
		services.set(IRemoteConnectionService, api.remoteConnections ?? UnavailableRemoteConnectionService);
		services.set(IRemoteTunnelService, api.remoteTunnels ?? UnavailableRemoteTunnelService);
		if (nativeHostApi) {
			services.set(INativeHostService, nativeHostApi);
		}
		const workspaceOpenService = new WorkspaceOpenService(nativeHostApi);
		services.set(IWorkspaceOpenService, workspaceOpenService);
		const workspaceContext = this.own(new WorkspaceContextService(workspace));
		this.workspaceContext = workspaceContext;
		services.set(IWorkspaceContextService, workspaceContext);
		const workspaceTrustService = new AppServerWorkspaceTrustService(api.workspaceTrust);
		services.set(IWorkspaceTrustService, workspaceTrustService);
		const fileService = new BrowserFileService({
			api: api.fs,
			resourceApi: api.resource,
			workspaceContextService: workspaceContext,
			onDidChange: listener => {
				const subscription = api.events.subscribe(event => {
					if (event.method === "fs/changed") listener(event.params as FsChanged);
				});
				return {
					dispose: () => subscription.dispose(),
					[Symbol.dispose]: () => subscription.dispose(),
				};
			},
		});
		this.own(fileService);
		services.set(IFileService, fileService);
		const textFileService = new TextFileService(fileService);
		services.set(ITextFileService, textFileService);
		const untitledTextEditorService = this.own(new BrowserUntitledTextEditorService());
		services.set(IUntitledTextEditorService, untitledTextEditorService);
		const workingCopyService = this.own(new BrowserWorkingCopyService());
		services.set(IWorkingCopyService, workingCopyService);
		const workingCopyBackups = this.own(new IndexedDbWorkingCopyBackupService(workspace.id));
		this.workingCopyBackups = workingCopyBackups;
		services.set(IWorkingCopyBackupService, workingCopyBackups);
		const textResourceStore = getBrowserTextResourceStore(textFileService);
		const textModelService = this.own(getBrowserTextModelService(textResourceStore));
		services.set(ITextModelService, textModelService);
		const workspaceEditService = this.own(new BrowserWorkspaceEditService(textModelService, workingCopyService, fileService));
		services.set(IWorkspaceEditService, workspaceEditService);
		const bulkEditService = this.own(new BrowserBulkEditService(workspaceEditService));
		services.set(IBulkEditService, bulkEditService);
		const textMateService = this.own(new BrowserTextMateService());
		services.set(ITextMateService, textMateService);
		const languageFeaturesService = this.own(new LanguageFeaturesService());
		services.set(ILanguageFeaturesService, languageFeaturesService);
		this.own(new AppServerLanguageProviders(languageFeaturesService, api.language, workspaceContext, { workspaceTrust: workspaceTrustService, events: api.events }));
		const codeIntelligenceDocuments = new AppServerCodeIntelligenceDocumentService(api.symbolIndex);
		services.set(ICodeIntelligenceDocumentService, codeIntelligenceDocuments);
		const languageDiagnosticsService = this.own(new AppServerLanguageDiagnosticsService(api.language, api.events, workspaceContext, codeIntelligenceDocuments, workspaceTrustService));
		services.set(ILanguageDiagnosticsService, languageDiagnosticsService);
		const extensionService = this.own(new AppServerExtensionService({ api: api.extensions, eventApi: api.events, textMateService, languageFeaturesService }));
		services.set(IExtensionService, extensionService);
		const extensionReady = extensionService.start();
		void extensionReady.catch(error => logService.error("extensions", "Declarative extension activation failed", error));
		services.set(
			IWorkspaceSearchService,
			new BrowserWorkspaceSearchService(api.workspaceSearch, workspaceContext),
		);
		const terminalService = this.own(new TerminalService(api.terminal, workspaceContext));
		services.set(ITerminalService, terminalService);
		const gitService = this.own(new GitService({ api: api.git, appServerApi: api.appServer, eventApi: api.events, workspaceContext }));
		services.set(IGitService, gitService);
		services.set(ICodeIndexService, new AppServerCodeIndexService(api.codeIndex));
		services.set(IConnectorService, this.own(new AppServerConnectorService(api.connectors, api.events)));
		services.set(IAccountService, this.own(new AppServerAccountService(api.accounts, api.events)));
		services.set(IPluginService, this.own(new AppServerPluginService(api.plugins, api.events)));
		const marketplaceService = this.own(new AppServerMarketplaceService(api.marketplace, api.events));
		services.set(IMarketplaceService, marketplaceService);
		services.set(IToolSearchService, new AppServerToolSearchService(api.toolSearch));
		const workbenchState = workspaceContext.getWorkbenchState();
		const workbenchWindow = this.own(new WorkbenchWindow({
			root: workbenchRoot,
			modeId,
			workbenchState,
		}));
		services.set(IWorkbenchHostService, workbenchWindow);
		const ownerDocument = workbenchWindow.ownerDocument;
		let workbenchLayout: WorkbenchLayout | undefined;
		const layoutService = this.own(new BrowserLayoutService({
			root: workbenchRoot,
			getContainerOffset: () => workbenchLayout?.mainContainerOffset ?? {
				top: 0,
				quickInputTop: 0,
			},
			focus: () => this.editor.focus(),
		}));
		services.set(ILayoutService, layoutService);

		const configuration = this.own(new WorkbenchConfigurationService({
			api: configurationApi,
		}));
		services.set(IConfigurationService, configuration);
		const chatService = this.own(new ChatService({ modelApi: api.model, threadApi: api.thread, turnApi: api.turn, skillApi: api.skills, appServerApi: api.appServer, eventApi: api.events, configurationService: configuration }));
		services.set(IChatService, chatService);
		services.set(IChatContextPickService, new ChatContextPickService());
		const languagePackService = this.own(new MarketplaceLanguagePackService(marketplaceService, builtinLanguagePackCatalogs));
		services.set(ILanguagePackService, languagePackService);
		const localeService = this.own(new WorkbenchLocaleService(configuration, languagePackService));
		services.set(ILocaleService, localeService);
		const localizationService = this.own(new WorkbenchLocalizationService(localeService, languagePackService));
		services.set(ILocalizationService, localizationService);
		const ownerWindow = ownerDocument.defaultView;
		if (!ownerWindow) {
			throw new Error("Workbench requires an owner window");
		}
		this.ownerWindow = ownerWindow;
		services.set(IClipboardService, new BrowserClipboardService(ownerWindow.navigator.clipboard));
		const lifecycleService = this.own(new BrowserLifecycleService({ ownerWindow, onError: error => logService.error("lifecycle", "Workbench shutdown failed", error) }));
		this.lifecycleService = lifecycleService;
		services.set(ILifecycleService, lifecycleService);
		services.set(IWorkbenchModeService, this.own(new WorkbenchModeService({
			currentModeId: modeId,
			configurationService: configuration,
			lifecycleService,
			switchHostMode: switchWorkbenchMode,
		})));
		const workingCopyBackupTracker = this.own(new WorkingCopyBackupTracker(workingCopyService, workingCopyBackups, ownerWindow));
		this.workingCopyBackupTracker = workingCopyBackupTracker;
		const storage = this.own(new BrowserStorageService({
			ownerWindow,
			applicationId: mode.storageNamespace,
			workspaceId: workspace.id,
		}));
		this.workbenchWindow = workbenchWindow;
		this.storage = storage;
		services.set(IStorageService, storage);
		const recentWorkspaces = this.own(new RecentWorkspacesService(storage, workspaceContext, workspaceOpenService));
		services.set(IRecentWorkspacesService, recentWorkspaces);
		this.own(lifecycleService.onWillShutdown(event => {
			event.join(workingCopyBackupTracker.flush(), "working-copy backup flush");
			event.join(storage.flush(WillSaveStateReason.SHUTDOWN), "Workbench storage flush");
		}));
		const outputService = this.own(new OutputService({ storageService: storage }));
		services.set(IOutputService, outputService);
		const systemOutputService = this.own(new SystemOutputService(outputService, api.appServer, workbenchWindow));
		this.own(logService.registerSink(systemOutputService));
		const serviceContributionReady: Promise<void>[] = [];
		installWorkbenchServiceContributions({ services, own: value => this.own(value), blockRestorationUntil: operation => serviceContributionReady.push(operation) });
		services.set(IAccessibleViewInformationService, this.own(new AccessibleViewInformationService(storage)));
		const themeService = this.own(new ThemeService(
			resolveWorkbenchColorTheme(
				configuration.getValue(WorkbenchConfiguration.colorTheme),
				ownerWindow.matchMedia("(prefers-color-scheme: dark)").matches,
			),
		));
		services.set(IThemeService, themeService);
		let textMateThemeRevision = 0;
		const updateTextMateTheme = (): void => {
			const model = textMateService.mutableScopeTheme;
			if (!model) return;
			const activeTheme = themeService.getColorTheme();
			try { model.replace(projectExtensionTokenTheme(extensionService.themes.currentCatalog, activeTheme.colorScheme, ++textMateThemeRevision, activeTheme.id)); }
			catch (error) { logService.error("theme", "Failed to apply extension token theme", error); }
		};
		this.own(extensionService.themes.onDidChange(() => updateTextMateTheme()));
		this.own(themeService.onDidColorThemeChange(() => updateTextMateTheme()));
		services.set(IUserThemeService, userThemeService ?? UnavailableUserThemeService);
		services.set(
			IFileIconThemeService,
			this.own(new SetiFileIconThemeService(themeService)),
		);
		const workbenchThemeController = this.own(new WorkbenchThemeController(
			configuration,
			themeService,
			ownerWindow,
		));
		this.own(extensionService.onDidChange(() => workbenchThemeController.refresh()));
		this.own(bindColorTheme(themeService, workbenchRoot));
		const statusbarService = this.own(new StatusbarService());
		services.set(IStatusbarService, statusbarService);
		const dialogService = this.own(new DialogService());
		services.set(IDialogService, dialogService);
		services.set(IDialogsModel, dialogService.model);
		const languageServerStatusService = this.own(new AppServerLanguageServerStatusService(api.events, dialogService, outputService, statusbarService, workspaceContext));
		services.set(ILanguageServerStatusService, languageServerStatusService);
		services.set(
			IWorkbenchDialogHandler,
			new BrowserDialogHandler(workbenchRoot),
		);
		const interactionServices = this.own(new WorkbenchInteractionServices({
			services,
			layoutService,
			configurationService: configuration,
			keybindingsResourceApi,
			keyboardLayoutProvider,
			userKeyboardLayoutApi,
			statusbarService,
			createContextMenuService,
		}));
		const commands = interactionServices.commandService;
		const contextKeys = interactionServices.contextKeyService;
		const menus = interactionServices.menuService;
		const contextViews = interactionServices.contextViewService;
		const contextMenus = interactionServices.contextMenuService;
		const accessibilityService = this.own(nativeHostApi
			? new NativeAccessibilityService({
				root: workbenchRoot,
				contextKeyService: contextKeys,
				configurationService: configuration,
				nativeHostApi,
			})
			: new AccessibilityService({
				root: workbenchRoot,
				contextKeyService: contextKeys,
				configurationService: configuration,
			}));
		services.set(IAccessibilityService, accessibilityService);
		this.own(bindWorkbenchContextKeys(contextKeys, workspaceContext, workingCopyService));
		const viewDescriptors = this.own(new ViewDescriptorService({
			contextKeyService: contextKeys,
		}));
		services.set(IViewDescriptorService, viewDescriptors);
		const sessionService = this.own(new AppServerSessionsManagementService({
			session: api.session,
			turn: api.turn,
			events: api.events,
			...(nativeHostApi ? {
				workspaceRouter: {
					currentWorkspaceRoot: () => workspaceOpenTarget(workspaceContext.getWorkspace()),
					reopenWorkspace: (root: string) => nativeHostApi.openWorkspace(root),
				},
			} : {}),
		}));
		services.set(ISessionsManagementService, sessionService);
		const keybindings = interactionServices.keybindingService;
		const contributions = this.own(
			WorkbenchContributionsRegistry.createHost(services),
		);
		contributions.advance(WorkbenchPhase.BlockStartup);

		const titlebar = this.own(createTitlebarPart(workbenchRoot, {
			menuService: menus,
			contextMenuService: contextMenus,
			localizationService,
		}));
		const sidebar = this.own(new SidebarPart(workbenchRoot, {
			viewDescriptorService: viewDescriptors,
			storageService: storage,
			localizationService,
			ariaLabelKey: { bundle: "zeta.regions", key: "primarySidebar" },
			viewsAriaLabelKey: { bundle: "zeta.regions", key: "primarySidebarViews" },
		}));
		const agentSidebar = this.own(new SidebarPart(workbenchRoot, {
			viewDescriptorService: viewDescriptors,
			storageService: storage,
			localizationService,
			id: "agentSidebar",
			location: ViewContainerLocation.AgentSidebar,
			ariaLabel: "Agent sidebar",
			ariaLabelKey: { bundle: "zeta.regions", key: "agentSidebar" },
			viewsAriaLabel: "Agent sidebar views",
			viewsAriaLabelKey: { bundle: "zeta.regions", key: "agentSidebarViews" },
			compositeBarContainerFilter: () => false,
			titleActions: {
				menuService: menus,
				contextMenuProvider: contextMenus,
				menuId: MenuId.AgentSidebarTitle,
			},
		}));
		const welcomeRecentProjects = () => recentWorkspaces.recentWorkspaces.map(project => ({
			name: project.name,
			path: project.path,
			...(workspaceOpenService.canOpenWorkspace ? {
				onOpen: () => recentWorkspaces.openWorkspace(project.root),
			} : {}),
		}));
		const editor = this.own(new EditorPart(workbenchRoot, {
			configurationService: configuration,
			contextKeyService: contextKeys,
			keybindingService: keybindings,
			keybindingsResourceService: services.get(IKeybindingsResourceService),
			keyboardLayoutService: services.get(IKeyboardLayoutService),
			fileService,
			textFileService,
			textMateService,
			languageFeaturesService,
			languageResolver: languageFeaturesService,
			diffApi: api.diff,
			instantiationService,
			syntaxApi: api.syntax,
			languageDiagnosticsService,
			documentCollaborationApi: api.documentCollaboration,
			serverEvents: api.events,
			workingCopyService,
			bulkEditService,
			createLineGutterDecorations: resource => createEditorLineGutterDecorations(resource, services),
			createDecorationSources: (resource, model) => createEditorDecorationSources({ accessor: services, diffApi: api.diff, model, resource }),
			saveAsResource: nativeHostApi
				? async (defaultName) => {
					const filePath = await nativeHostApi.saveFile({ defaultName });
					return filePath ? URI.file(filePath) : undefined;
				}
				: undefined,
			titleActions: {
				menuService: menus,
				contextMenuProvider: contextMenus,
			},
			welcomeVisible: workbenchState === WorkbenchState.EMPTY,
			welcome: {
				productName: mode.title,
				recentProjects: welcomeRecentProjects(),
				actions: {
					openFolder: workspaceOpenService.canOpenFolder
						? () => workspaceOpenService.openFolder()
						: undefined,
					connectViaSsh: () => commands.executeCommand(ConnectToRemoteCommandId),
				},
			},
		}));
		this.own(bindEditorContextKeys(contextKeys, editor, EditorPanes, languageFeaturesService));
		services.set(IEditorPart, editor);
		services.set(IEditorService, new BrowserEditorService(editor));
		const openSidebarComposite = (
			compositeId: string,
		): PaneComposite => {
			const viewContainer = viewDescriptors
				.getViewContainers(ViewContainerLocation.Sidebar)
				.find((candidate) => candidate.id === compositeId);
			if (!viewContainer) {
				throw new Error(
					`Sidebar Composite is not registered: ${compositeId}`,
				);
			}
			if (!sidebar.getComposite(viewContainer.id)) {
				sidebar.addComposite(new PaneComposite(sidebar.domNode, {
					viewContainer,
					model: viewDescriptors.getViewContainerModel(viewContainer.id),
					instantiationService,
					contextKeyService: contextKeys,
					localizationService,
				}));
			}
			sidebar.showComposite(viewContainer.id);
			const composite = sidebar.getComposite(viewContainer.id);
			assertDefined(composite, `Sidebar Composite is not available: ${viewContainer.id}`);
			return composite;
		};
		const openAgentSidebarComposite = (
			compositeId: string,
		): PaneComposite => {
			const viewContainer = viewDescriptors
				.getViewContainers(ViewContainerLocation.AgentSidebar)
				.find((candidate) => candidate.id === compositeId);
			if (!viewContainer) {
				throw new Error(
					`Agent Sidebar Composite is not registered: ${compositeId}`,
				);
			}
			if (!agentSidebar.getComposite(viewContainer.id)) {
				agentSidebar.addComposite(new PaneComposite(agentSidebar.domNode, {
					viewContainer,
					model: viewDescriptors.getViewContainerModel(viewContainer.id),
					instantiationService,
					contextKeyService: contextKeys,
					localizationService,
				}));
			}
			agentSidebar.showComposite(viewContainer.id);
			const composite = agentSidebar.getComposite(viewContainer.id);
			assertDefined(composite, `Agent Sidebar Composite is not available: ${viewContainer.id}`);
			return composite;
		};
		const panel = this.own(new PanelPart(workbenchRoot, {
			viewDescriptorService: viewDescriptors,
			storageService: storage,
			localizationService,
			contextMenuProvider: contextMenus,
			titleActions: {
				menuService: menus,
				contextMenuProvider: contextMenus,
				menuId: MenuId.PanelTitle,
			},
		}));
		this.editor = editor;
		this.own(recentWorkspaces.onDidChange(() => {
			editor.setWelcomeRecentProjects(welcomeRecentProjects());
		}));
		const openPanelComposite = (
			compositeId: string,
		): PaneComposite => {
			const viewContainer = viewDescriptors
				.getViewContainers(ViewContainerLocation.Panel)
				.find((candidate) => candidate.id === compositeId);
			if (!viewContainer) {
				throw new Error(
					`Panel Composite is not registered: ${compositeId}`,
				);
			}
			if (!panel.getComposite(viewContainer.id)) {
				panel.addComposite(new PaneComposite(panel.domNode, {
					viewContainer,
					model: viewDescriptors.getViewContainerModel(viewContainer.id),
					instantiationService,
					contextKeyService: contextKeys,
					localizationService,
					paneHeaders: "hidden",
					paneLayout: "fill",
				}));
			}
			panel.showComposite(viewContainer.id);
			const composite = panel.getComposite(viewContainer.id);
			assertDefined(composite, `Panel Composite is not available: ${viewContainer.id}`);
			return composite;
		};
		const auxiliarybar = this.own(new AuxiliarybarPart(workbenchRoot, {
			viewDescriptorService: viewDescriptors,
			storageService: storage,
			localizationService,
		}));
		const statusbar = this.own(new StatusbarPart(workbenchRoot, statusbarService));
		this.own(bindWorkbenchActiveCompositeContextKeys(contextKeys, {
			sidebar,
			auxiliarybar,
			agentSidebar,
			panel,
		}));

		const parts = new Map<WorkbenchPartId, WorkbenchPart>([
			["titlebar", titlebar],
			["statusbar", statusbar],
			["sidebar", sidebar],
			["auxiliarybar", auxiliarybar],
			["agentSidebar", agentSidebar],
			["editor", editor],
			["panel", panel],
		]);
		const layout = this.own(new WorkbenchLayout(workbenchRoot, parts, {
			initialDimension: layoutService.mainContainerDimension,
			fallbackPartVisibility: DEFAULT_WORKBENCH_LAYOUT.parts,
			defaultLayout,
			storageService: storage,
		}));
		workbenchLayout = layout;
		this.workbenchLayout = layout;
		services.set(IWorkbenchLayoutService, layout);
		this.own(bindResizableLayout(layoutService.onDidLayoutMainContainer, layout));
		const openAuxiliaryComposite = (compositeId: string): PaneComposite => {
			const viewContainer = viewDescriptors
				.getViewContainers(ViewContainerLocation.AuxiliaryBar)
				.find((candidate) => candidate.id === compositeId);
			if (!viewContainer) {
				throw new Error(
					`Auxiliary Bar Composite is not registered: ${compositeId}`,
				);
			}
			if (!auxiliarybar.getComposite(viewContainer.id)) {
				auxiliarybar.addComposite(new PaneComposite(auxiliarybar.domNode, {
					viewContainer,
					model: viewDescriptors.getViewContainerModel(viewContainer.id),
					instantiationService,
					contextKeyService: contextKeys,
					localizationService,
					paneHeaders: "hidden",
					paneLayout: "fill",
				}));
			}
			auxiliarybar.showComposite(viewContainer.id);
			const composite = auxiliarybar.getComposite(viewContainer.id);
			assertDefined(composite, `Auxiliary Bar Composite is not available: ${viewContainer.id}`);
			return composite;
		};
		this.restoreActiveViewContainers = () => {
			openSidebarComposite(requiredViewContainerToRestore(
				viewDescriptors,
				ViewContainerLocation.Sidebar,
				sidebar.getCompositeIdToRestore(),
			).id);
			// Fixed Panel and Auxiliary Bar views may depend on the host layout during construction.
			openPanelComposite(requiredViewContainerToRestore(
				viewDescriptors,
				ViewContainerLocation.Panel,
				panel.getCompositeIdToRestore(),
			).id);
			openAuxiliaryComposite(requiredViewContainerToRestore(
				viewDescriptors,
				ViewContainerLocation.AuxiliaryBar,
				auxiliarybar.getCompositeIdToRestore(),
			).id);
			if (layout.isPartVisible("agentSidebar")) {
				openAgentSidebarComposite(requiredViewContainerToRestore(
					viewDescriptors,
					ViewContainerLocation.AgentSidebar,
					agentSidebar.getCompositeIdToRestore(),
				).id);
			}
		};
		this.restoreActiveViewContainers();
		const viewsService = new ViewsService({
			viewDescriptorService: viewDescriptors,
			openViewContainer: (container) => {
				switch (container.location) {
					case ViewContainerLocation.Sidebar:
						layout.showPart("sidebar");
						return openSidebarComposite(container.id);
					case ViewContainerLocation.AuxiliaryBar:
						layout.showPart("auxiliarybar");
						return openAuxiliaryComposite(container.id);
					case ViewContainerLocation.AgentSidebar:
						layout.showPart("agentSidebar");
						return openAgentSidebarComposite(container.id);
					case ViewContainerLocation.Panel:
						layout.showPart("panel");
						return openPanelComposite(container.id);
				}
			},
		});
		services.set(IViewsService, viewsService);
		this.own(outputService.onDidRequestShowChannel(request => {
			if (request.focus === "take") viewsService.focusView(OUTPUT_VIEW_ID);
			else viewsService.openView(OUTPUT_VIEW_ID);
		}));
		this.own(bindWorkbenchPartVisibilityContextKeys(contextKeys, layout));
		this.own(sidebar.onDidSelectComposite(
			({ compositeId }) => {
				if (sidebar.activeCompositeId === compositeId) return;
				openSidebarComposite(compositeId);
			},
		));
		this.own(agentSidebar.onDidSelectComposite(
			({ compositeId }) => {
				if (agentSidebar.activeCompositeId === compositeId) return;
				openAgentSidebarComposite(compositeId);
			},
		));
		this.own(panel.onDidSelectComposite(
			({ compositeId }) => {
				if (panel.activeCompositeId === compositeId) return;
				openPanelComposite(compositeId);
			},
		));
		void sessionService.initialize();
		contributions.advance(WorkbenchPhase.BlockRestore);
		layoutService.layout();
		this.whenRestored = this.completeStartupRestoration([extensionReady, ...serviceContributionReady], workingCopyBackups, editor, contributions);
	}

	shutdown(reason: ShutdownReason): Promise<void> {
		return this.lifecycleService.shutdown(reason);
	}

	private async completeStartupRestoration(extensionReady: readonly Promise<void>[], backups: IWorkingCopyBackupService, editor: EditorPart, contributions: WorkbenchContributionHost): Promise<void> {
		await Promise.allSettled(extensionReady);
		if (this.isDisposed) return;
		await this.restoreWorkingCopyBackups(backups, editor);
		if (this.isDisposed) return;
		contributions.advance(WorkbenchPhase.AfterRestored);
		this.own(disposableWindowTimeout(this.ownerWindow, () => contributions.advance(WorkbenchPhase.Eventually), 2_000));
	}

	private async restoreWorkingCopyBackups(backups: IWorkingCopyBackupService, editor: EditorPart): Promise<void> {
		let pending: readonly WorkingCopyBackup[];
		try { pending = await backups.list(); }
		catch (error) { this.logService.error("workingCopy", "Failed to list working-copy backups", error); return; }
		for (const backup of pending) {
			try {
				let pane;
				try {
					pane = await editor.openEditor({ resource: backup.resource, ...(backup.languageId ? { languageId: backup.languageId } : {}), ...(backup.contentType ? { contentType: backup.contentType } : {}), ...(backup.label ? { label: backup.label } : {}) });
				} catch {
					pane = await editor.openEditor({ resource: backup.resource, initialText: "", ...(backup.languageId ? { languageId: backup.languageId } : {}), ...(backup.contentType ? { contentType: backup.contentType } : {}), ...(backup.label ? { label: backup.label } : {}) });
				}
				const workingCopy = pane.workingCopy;
				if (!workingCopy || workingCopy.backupKind !== backup.kind) throw new Error(`Restored editor does not support ${backup.kind} backups`);
				workingCopy.restoreBackup(backup.content);
			} catch (error) {
				this.logService.error("workingCopy", `Failed to restore working-copy backup '${backup.resource.toString()}'`, error);
			}
		}
	}

	/** Applies a host-authoritative workspace replacement without rebuilding the Workbench. */
	updateWorkspace(workspace: IWorkspace): Promise<void> {
		const switching = this.workspaceSwitchQueue.then(() => this.doUpdateWorkspace(workspace));
		this.workspaceSwitchQueue = switching.then(() => undefined, () => undefined);
		return switching;
	}

	private async doUpdateWorkspace(workspace: IWorkspace): Promise<void> {
		if (this.workspaceContext.getWorkspace().id === workspace.id) return;
		await this.workingCopyBackupTracker.flush();
		for (const group of this.editor.groups) {
			for (const input of [...group.inputs]) group.closeEditor(input);
		}
		await this.workingCopyBackupTracker.flush();
		this.workingCopyBackups.switchWorkspace(workspace.id);
		await this.storage.flush(WillSaveStateReason.WORKSPACE_CHANGE);
		this.storage.switchWorkspace(workspace.id);
		const nextWorkbenchState = workbenchStateFromWorkspace(workspace);
		this.workbenchWindow.setWorkbenchState(nextWorkbenchState);
		this.workspaceContext.updateWorkspace(workspace);
		this.editor.setWelcomeVisible(nextWorkbenchState === WorkbenchState.EMPTY);
		this.workbenchLayout.restoreWorkspaceState();
		this.restoreActiveViewContainers?.();
		await this.restoreWorkingCopyBackups(this.workingCopyBackups, this.editor);
	}
}

function requiredViewContainerToRestore(
	service: IViewDescriptorService,
	location: ViewContainerLocation,
	containerId: string | undefined,
) {
	const container = service
		.getViewContainers(location)
		.find((candidate) => candidate.id === containerId);
	if (!container) {
		throw new Error(
			`Workbench has no ${location} view container to restore`,
		);
	}
	return container;
}
