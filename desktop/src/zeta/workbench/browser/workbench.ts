import "./style.js";
import type { FsChanged } from "../../../../generated/app-server/types.js";
import { bindResizableLayout } from "../../base/browser/ui/resizable/resizable.js";
import {
  type IDisposable,
  DisposableOwner,
} from "../../base/common/lifecycle.js";
import { assertDefined } from "../../base/common/types.js";
import type {
  ProductConfiguration,
} from "../../product/common/product.js";
import { URI } from "../../base/common/uri.js";
import type { AppServerConnectionState } from "../../platform/app-server/common/appServerApi.js";
import { AccessibilityService } from "../../platform/accessibility/browser/accessibilityService.js";
import { IAccessibilityService } from "../../platform/accessibility/common/accessibility.js";
import type { IRendererHost } from "../../platform/renderer/common/rendererHost.js";
import type {
  INativeHostApi,
} from "../../platform/native/common/nativeHost.js";
import { MenuId } from "../../platform/actions/common/actions.js";
import {
  type IConfigurationApi,
  IConfigurationService,
} from "../../platform/configuration/common/configuration.js";
import { IStorageService, WillSaveStateReason } from "../../platform/storage/common/storage.js";
import { BrowserLayoutService } from "../../platform/layout/browser/layoutService.js";
import { ILayoutService } from "../../platform/layout/common/layoutService.js";
import "../../platform/layout/browser/zIndexRegistry.js";
import {
  InstantiationService,
  ServiceCollection,
} from "../../platform/instantiation/common/instantiation.js";
import type { IKeybindingsResourceApi } from "../../platform/keybinding/common/keybindingsResource.js";
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
import { type IAnyWorkspaceIdentifier, IWorkspaceContextService, workbenchStateFromWorkspaceIdentifier } from "../../platform/workspace/common/workspace.js";
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
  StatusbarAlignment,
  StatusbarService,
} from "../services/statusbar/browser/statusbar.js";
import {
  WorkspaceContextService,
} from "../services/workspaces/browser/workspaceContextService.js";
import {
  IWorkspaceOpenService,
  WorkspaceOpenService,
} from "../services/workspaces/browser/workspaceOpenService.js";
import {
  IViewDescriptorService,
  ViewDescriptorService,
} from "../services/views/common/viewDescriptorService.js";
import {
  IViewsService,
  ViewsService,
} from "../services/views/browser/viewsService.js";
import { WorkbenchSessionService } from "../services/sessions/browser/sessionService.js";
import { IWorkbenchSessionService } from "../services/sessions/common/sessionService.js";
import {
  WorkbenchConfigurationService,
} from "../services/configuration/browser/configurationService.js";
import type {
  WorkbenchContextMenuServiceFactory,
} from "../services/contextmenu/common/contextMenuService.js";
import {
  DialogService,
} from "../services/dialogs/common/dialogService.js";
import {
  bindWorkbenchContextKeys,
  bindWorkbenchPartVisibilityContextKeys,
} from "./contextkeys.js";
import { WorkbenchThemeController } from "./theme.js";
import { WorkbenchLayout } from "./layout.js";
import { IWorkbenchLayoutService, type WorkbenchPartId } from "../services/layout/browser/layoutService.js";
import { BrowserStorageService } from "../services/storage/browser/storageService.js";
import { IWorkspaceSearchService } from "../../platform/search/common/search.js";
import { BrowserWorkspaceSearchService } from "../../platform/search/browser/searchService.js";
import type { WorkbenchPart } from "./part.js";
import { AuxiliarybarPart } from "./parts/auxiliarybar/auxiliarybarPart.js";
import { EditorPart, IEditorPart } from "./parts/editor/editorPart.js";
import { PanelPart } from "./parts/panel/panelPart.js";
import { SidebarPart } from "./parts/sidebar/sidebarPart.js";
import { StatusbarPart } from "./parts/statusbar/statusbarPart.js";
import type {
  TitlebarPartFactory,
} from "./parts/titlebar/titlebarPart.js";
import { PaneComposite } from "./parts/views/paneComposite.js";
import { IWorkbenchWindowService, WorkbenchWindow } from "./window.js";
import { TerminalService } from "../services/terminal/browser/terminalService.js";
import { ITerminalService } from "../services/terminal/common/terminal.js";
import { ITextFileService, TextFileService } from "../services/textfile/common/textFileService.js";
import { ITextMateService } from "../services/textMate/common/textMateService.js";
import { BrowserTextMateService } from "../services/textMate/browser/browserTextMateService.js";
import { AppServerExtensionService } from "../services/extensions/browser/appServerExtensionService.js";
import { IExtensionService } from "../services/extensions/common/extensionService.js";
import { ILanguageFeaturesService, LanguageFeaturesService } from "../services/language/common/languageFeaturesService.js";
import { GitService } from "../services/git/browser/gitService.js";
import { IGitService } from "../services/git/common/gitService.js";
import { ChatService } from "../services/chat/browser/chatService.js";
import { IChatService } from "../services/chat/common/chatService.js";
import { ICodeIndexService } from "../../platform/codeIndex/common/codeIndexService.js";
import { AppServerCodeIndexService } from "../services/codeIndex/browser/appServerCodeIndexService.js";
import { IToolSearchService } from "../../platform/toolSearch/common/toolSearchService.js";
import { IConnectorService } from "../../platform/connectors/common/connectorService.js";
import { AppServerConnectorService } from "../services/connectors/browser/appServerConnectorService.js";
import { IPluginService } from "../../platform/plugins/common/pluginService.js";
import { ILanguageMarketplaceService } from "../../platform/language/common/languageMarketplaceService.js";
import { AppServerPluginService } from "../services/plugins/browser/appServerPluginService.js";
import { AppServerLanguageMarketplaceService } from "../services/language/browser/appServerLanguageMarketplaceService.js";
import { AppServerToolSearchService } from "../services/toolSearch/browser/appServerToolSearchService.js";
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
import { getBrowserTextModelService } from "../../editor/browser/services/browserTextModelService.js";
import { getBrowserTextResourceStore } from "../contrib/codeEditor/browser/browserTextResourceStore.js";
import { AppServerLanguageProviders } from "../services/language/browser/appServerLanguageProviders.js";
import { AppServerLanguageDiagnosticsService } from "../services/language/browser/appServerLanguageDiagnosticsService.js";
import { AppServerLanguageServerStatusService } from "../services/language/browser/appServerLanguageServerStatusService.js";
import { ILanguageServerStatusService } from "../services/language/common/languageServerStatusService.js";
import { ILanguageDiagnosticsService } from "../services/language/common/languageDiagnosticsService.js";
import { createWorkbenchSession, type WorkbenchSession } from "./workbenchSession.js";
import { createEditorLineGutterDecorations } from "./parts/editor/editorGutterDecorations.js";
import { installWorkbenchServiceContributions } from "./workbenchServiceContributions.js";
import { WorkbenchInteractionServices } from "./workbenchInteractionServices.js";
import { AppServerConnectionStateObserver } from "./appServerConnectionStateObserver.js";

/** Host-specific inputs required to construct a workbench. */
export interface IStartWorkbenchOptions {
  readonly product: ProductConfiguration;
  readonly session: WorkbenchSession;
  readonly api: IRendererHost;
  readonly container: HTMLElement | null;
  readonly workspace: IAnyWorkspaceIdentifier;
  readonly configurationApi?: IConfigurationApi;
  readonly keybindingsResourceApi?: IKeybindingsResourceApi;
  readonly nativeHostApi?: INativeHostApi;
  readonly userThemeService?: IUserThemeServiceContract;
  readonly createContextMenuService: WorkbenchContextMenuServiceFactory;
  readonly createTitlebarPart: TitlebarPartFactory;
}

/** Starts the browser workbench and binds its commands to the initial UI. */
export function startWorkbench({
  product,
  session,
  api,
  container,
  workspace,
  configurationApi,
  keybindingsResourceApi,
  nativeHostApi,
  userThemeService,
  createContextMenuService,
  createTitlebarPart,
}: IStartWorkbenchOptions): Workbench {
  return new Workbench(
    product,
    session,
    api,
    container ?? document.body,
    workspace,
    configurationApi,
    keybindingsResourceApi,
    nativeHostApi,
    userThemeService,
    createContextMenuService,
    createTitlebarPart,
  );
}

/** Owns the renderer workbench, its parts, commands, and runtime layout. */
export class Workbench extends DisposableOwner {
  readonly session: WorkbenchSession;
  /** Resolves after dirty working copies are restored and AfterRestored contributions are active. */
  readonly whenRestored: Promise<void>;
  private readonly workspaceContext: WorkspaceContextService;
  private readonly storage: BrowserStorageService;
  private readonly editor: EditorPart;
  private readonly workingCopyBackups: IndexedDbWorkingCopyBackupService;
  private readonly workingCopyBackupTracker: WorkingCopyBackupTracker;
  private readonly workbenchWindow: WorkbenchWindow;
  private workspaceSwitchQueue: Promise<void> = Promise.resolve();
  private disposed = false;

  constructor(
    product: ProductConfiguration,
    session: WorkbenchSession,
    api: IRendererHost,
    workbenchRoot: HTMLElement,
    workspace: IAnyWorkspaceIdentifier,
    configurationApi: IConfigurationApi | undefined,
    keybindingsResourceApi: IKeybindingsResourceApi | undefined,
    nativeHostApi: INativeHostApi | undefined,
    userThemeService: IUserThemeServiceContract | undefined,
    createContextMenuService: WorkbenchContextMenuServiceFactory,
    createTitlebarPart: TitlebarPartFactory,
  ) {
    super();
    const normalizedSession = createWorkbenchSession(session);
    if (normalizedSession.productId !== product.id) {
      throw new TypeError(
        `Workbench session '${normalizedSession.id}' belongs to '${normalizedSession.productId}', not '${product.id}'`,
      );
    }
    this.session = normalizedSession;
    const services = new ServiceCollection();
    const instantiationService = new InstantiationService(services);
    if (nativeHostApi) {
      services.set(INativeHostService, nativeHostApi);
    }
    services.set(
      IWorkspaceOpenService,
      new WorkspaceOpenService(nativeHostApi),
    );
    const workspaceContext = this.own(new WorkspaceContextService(workspace));
    this.workspaceContext = workspaceContext;
    services.set(IWorkspaceContextService, workspaceContext);
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
    const workspaceEditService = this.own(new BrowserWorkspaceEditService(getBrowserTextModelService(textResourceStore), workingCopyService, fileService));
    services.set(IWorkspaceEditService, workspaceEditService);
    const textMateService = this.own(new BrowserTextMateService());
    services.set(ITextMateService, textMateService);
    const languageFeaturesService = this.own(new LanguageFeaturesService());
    services.set(ILanguageFeaturesService, languageFeaturesService);
    this.own(new AppServerLanguageProviders(languageFeaturesService, api.language, workspaceContext));
    const languageDiagnosticsService = this.own(new AppServerLanguageDiagnosticsService(api.language, api.events, workspaceContext));
    services.set(ILanguageDiagnosticsService, languageDiagnosticsService);
    const extensionService = this.own(new AppServerExtensionService({ api: api.extensions, eventApi: api.events, textMateService, languageFeaturesService }));
    services.set(IExtensionService, extensionService);
    const extensionReady = extensionService.start();
    void extensionReady.catch(error => console.error("Declarative extension activation failed", error));
    services.set(
      IWorkspaceSearchService,
      new BrowserWorkspaceSearchService(api.workspaceSearch),
    );
    const terminalService = this.own(new TerminalService(api.terminal));
    services.set(ITerminalService, terminalService);
    const gitService = this.own(new GitService({ api: api.git, appServerApi: api.appServer, eventApi: api.events }));
    services.set(IGitService, gitService);
    const chatService = this.own(new ChatService({ modelApi: api.model, threadApi: api.thread, turnApi: api.turn, skillApi: api.skills, appServerApi: api.appServer, eventApi: api.events }));
    services.set(IChatService, chatService);
    services.set(ICodeIndexService, new AppServerCodeIndexService(api.codeIndex));
    services.set(IConnectorService, this.own(new AppServerConnectorService(api.connectors, api.events)));
    services.set(IPluginService, this.own(new AppServerPluginService(api.plugins, api.events)));
    services.set(ILanguageMarketplaceService, new AppServerLanguageMarketplaceService(api.language));
    services.set(IToolSearchService, new AppServerToolSearchService(api.toolSearch));
    const workbenchState = workspaceContext.getWorkbenchState();
    const workbenchWindow = this.own(new WorkbenchWindow({
      root: workbenchRoot,
      productId: product.id,
      workbenchState,
    }));
    services.set(IWorkbenchWindowService, workbenchWindow);
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
    const ownerWindow = ownerDocument.defaultView;
    if (!ownerWindow) {
      throw new Error("Workbench requires an owner window");
    }
    const workingCopyBackupTracker = this.own(new WorkingCopyBackupTracker(workingCopyService, workingCopyBackups, ownerWindow));
    this.workingCopyBackupTracker = workingCopyBackupTracker;
    const pageHideBackup = () => { void workingCopyBackupTracker.flush(); };
    ownerWindow.addEventListener("pagehide", pageHideBackup);
    this.defer(() => ownerWindow.removeEventListener("pagehide", pageHideBackup));
    const storage = this.own(new BrowserStorageService({
      ownerWindow,
      applicationId: product.storageNamespace,
      workspaceId: workspace.id,
    }));
    this.workbenchWindow = workbenchWindow;
    this.storage = storage;
    services.set(IStorageService, storage);
    const serviceContributionReady: Promise<void>[] = [];
    installWorkbenchServiceContributions({ services, rendererHost: api, fileService, workspaceContext, terminalService, storageService: storage, own: value => this.own(value), blockRestorationUntil: operation => serviceContributionReady.push(operation) });
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
      catch (error) { console.error("Failed to apply extension token theme", error); }
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
    const connectionStatus = this.own(statusbarService.addEntry(
      appServerStatusEntry("starting"),
      {
        id: "zeta.status.appServer",
        alignment: StatusbarAlignment.Left,
      },
    ));
    let previousConnectionState: AppServerConnectionState | undefined;
    const handleConnectionState = (state: AppServerConnectionState): void => {
      connectionStatus.update(appServerStatusEntry(state));
      const previous = previousConnectionState;
      previousConnectionState = state;
      if (state === "ready" && previous !== undefined && previous !== "ready") {
        void extensionService.reload().catch(error => console.error("Declarative extension refresh after App Server recovery failed", error));
      }
    };
    this.own(new AppServerConnectionStateObserver({
      api: api.appServer,
      onState: handleConnectionState,
      onReadError: (error: unknown) => {
        console.error("Failed to read App Server connection state", error);
        handleConnectionState("crashed");
      },
    }));
    const dialogService = this.own(new DialogService());
    services.set(IDialogService, dialogService);
    services.set(IDialogsModel, dialogService.model);
    const languageServerStatusService = this.own(new AppServerLanguageServerStatusService(api.events, dialogService, statusbarService));
    services.set(ILanguageServerStatusService, languageServerStatusService);
    services.set(
      IWorkbenchDialogHandler,
      new BrowserDialogHandler(workbenchRoot),
    );
    const interactionServices = this.own(new WorkbenchInteractionServices({
      services,
      ownerDocument,
      layoutService,
      configurationService: configuration,
      keybindingsResourceApi,
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
        ownerDocument,
        root: workbenchRoot,
        contextKeyService: contextKeys,
        configurationService: configuration,
        nativeHostApi,
      })
      : new AccessibilityService({
        ownerDocument,
        root: workbenchRoot,
        contextKeyService: contextKeys,
        configurationService: configuration,
      }));
    services.set(IAccessibilityService, accessibilityService);
    this.own(bindWorkbenchContextKeys(contextKeys, workspaceContext));
    const viewDescriptors = this.own(new ViewDescriptorService({
      contextKeyService: contextKeys,
    }));
    services.set(IViewDescriptorService, viewDescriptors);
    const sessionService = this.own(new WorkbenchSessionService({ session: api.session, events: api.events }));
    services.set(IWorkbenchSessionService, sessionService);
    const keybindings = interactionServices.keybindingService;
    const contributions = this.own(
      WorkbenchContributionsRegistry.createHost(services),
    );
    contributions.advance(WorkbenchPhase.BlockStartup);

    const titlebar = this.own(createTitlebarPart({
      menuService: menus,
      contextMenuService: contextMenus,
      ownerDocument,
    }));
    const sidebar = this.own(new SidebarPart({
      ownerDocument,
      viewDescriptorService: viewDescriptors,
    }));
    const agentSidebar = this.own(new SidebarPart({
      ownerDocument,
      viewDescriptorService: viewDescriptors,
      id: "agentSidebar",
      location: ViewContainerLocation.AgentSidebar,
      ariaLabel: "Agent sidebar",
      viewsAriaLabel: "Agent sidebar views",
      compositeBarContainerFilter: () => false,
      titleActions: {
        menuService: menus,
        contextMenuProvider: contextMenus,
        menuId: MenuId.AgentSidebarTitle,
      },
    }));
    const editor = this.own(new EditorPart(ownerDocument, {
      configurationService: configuration,
      keybindingService: keybindings,
      fileService,
      textFileService,
      textMateService,
      languageFeaturesService,
      languageResolver: languageFeaturesService,
      diffApi: api.diff,
      syntaxApi: api.syntax,
      languageDiagnosticsService,
      documentCollaborationApi: api.documentCollaboration,
      serverEvents: api.events,
      workingCopyService,
      workspaceEditService,
      createLineGutterDecorations: resource => createEditorLineGutterDecorations(resource, services),
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
    }));
    services.set(IEditorPart, editor);
    const sidebarCompositeDescriptor = requiredViewContainer(
      viewDescriptors,
      ViewContainerLocation.Sidebar,
      normalizedSession.composition.sidebar,
    );
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
        sidebar.addComposite(new PaneComposite({
          viewContainer,
          model: viewDescriptors.getViewContainerModel(viewContainer.id),
          instantiationService,
          contextKeyService: contextKeys,
          ownerDocument,
        }));
      }
      sidebar.showComposite(viewContainer.id);
      sidebar.setActiveComposite(viewContainer.id);
      const composite = sidebar.getComposite(viewContainer.id);
      assertDefined(composite, `Sidebar Composite is not available: ${viewContainer.id}`);
      return composite;
    };
    openSidebarComposite(sidebarCompositeDescriptor.id);
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
        agentSidebar.addComposite(new PaneComposite({
          viewContainer,
          model: viewDescriptors.getViewContainerModel(viewContainer.id),
          instantiationService,
          contextKeyService: contextKeys,
          ownerDocument,
        }));
      }
      agentSidebar.showComposite(viewContainer.id);
      agentSidebar.setActiveComposite(viewContainer.id);
      const composite = agentSidebar.getComposite(viewContainer.id);
      assertDefined(composite, `Agent Sidebar Composite is not available: ${viewContainer.id}`);
      return composite;
    };
    const panel = this.own(new PanelPart({
      ownerDocument,
      viewDescriptorService: viewDescriptors,
      contextMenuProvider: contextMenus,
    }));
    this.editor = editor;
    const panelCompositeDescriptor = requiredViewContainer(
      viewDescriptors,
      ViewContainerLocation.Panel,
      normalizedSession.composition.panel,
    );
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
        panel.addComposite(new PaneComposite({
          viewContainer,
          model: viewDescriptors.getViewContainerModel(viewContainer.id),
          instantiationService,
          contextKeyService: contextKeys,
          ownerDocument,
          paneHeaders: "hidden",
          paneLayout: "fill",
        }));
      }
      panel.showComposite(viewContainer.id);
      panel.setActiveComposite(viewContainer.id);
      const composite = panel.getComposite(viewContainer.id);
      assertDefined(composite, `Panel Composite is not available: ${viewContainer.id}`);
      return composite;
    };
    const auxiliarybar = this.own(new AuxiliarybarPart({
      ownerDocument,
      viewDescriptorService: viewDescriptors,
    }));
    const auxiliaryViewContainer = requiredViewContainer(
      viewDescriptors,
      ViewContainerLocation.AuxiliaryBar,
      normalizedSession.composition.auxiliarybar,
    );
    const statusbar = this.own(new StatusbarPart(
      statusbarService,
      ownerDocument,
    ));

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
      session: normalizedSession,
      storageService: storage,
    }));
    workbenchLayout = layout;
    services.set(IWorkbenchLayoutService, layout);
    this.own(bindResizableLayout(layoutService.onDidLayoutMainContainer, layout));
    // Fixed Panel and Auxiliary Bar views may depend on the host layout during construction.
    openPanelComposite(panelCompositeDescriptor.id);
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
        auxiliarybar.addComposite(new PaneComposite({
          viewContainer,
          model: viewDescriptors.getViewContainerModel(viewContainer.id),
          instantiationService,
          contextKeyService: contextKeys,
          ownerDocument,
          paneHeaders: "hidden",
          paneLayout: "fill",
        }));
      }
      auxiliarybar.showComposite(viewContainer.id);
      auxiliarybar.setActiveComposite(viewContainer.id);
      const composite = auxiliarybar.getComposite(viewContainer.id);
      assertDefined(composite, `Auxiliary Bar Composite is not available: ${viewContainer.id}`);
      return composite;
    };
    openAuxiliaryComposite(auxiliaryViewContainer.id);
    if (normalizedSession.layout.agentSidebar.visible) {
      openAgentSidebarComposite(normalizedSession.composition.agentSidebar);
    }
    services.set(IViewsService, new ViewsService({
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
    this.defer(() => { this.disposed = true; });
    void sessionService.initialize();
    contributions.advance(WorkbenchPhase.BlockRestore);
    layoutService.layout();
    this.whenRestored = this.completeStartupRestoration([extensionReady, ...serviceContributionReady], workingCopyBackups, editor, contributions);
    this.defer(() => {
      void storage.flush(WillSaveStateReason.SHUTDOWN);
    });
  }

  private async completeStartupRestoration(extensionReady: readonly Promise<void>[], backups: IWorkingCopyBackupService, editor: EditorPart, contributions: WorkbenchContributionHost): Promise<void> {
    await Promise.allSettled(extensionReady);
    if (this.disposed) return;
    await this.restoreWorkingCopyBackups(backups, editor);
    if (this.disposed) return;
    contributions.advance(WorkbenchPhase.AfterRestored);
    const eventuallyTimer = globalThis.setTimeout(() => contributions.advance(WorkbenchPhase.Eventually), 2_000);
    this.defer(() => globalThis.clearTimeout(eventuallyTimer));
  }

  private async restoreWorkingCopyBackups(backups: IWorkingCopyBackupService, editor: EditorPart): Promise<void> {
    let pending: readonly WorkingCopyBackup[];
    try { pending = await backups.list(); }
    catch (error) { console.error("Failed to list working-copy backups", error); return; }
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
        console.error(`Failed to restore working-copy backup '${backup.resource.toString()}'`, error);
      }
    }
  }

  /** Applies a host-authoritative workspace replacement without rebuilding the Workbench. */
  updateWorkspace(workspace: IAnyWorkspaceIdentifier): Promise<void> {
    const switching = this.workspaceSwitchQueue.then(() => this.doUpdateWorkspace(workspace));
    this.workspaceSwitchQueue = switching.then(() => undefined, () => undefined);
    return switching;
  }

  private async doUpdateWorkspace(workspace: IAnyWorkspaceIdentifier): Promise<void> {
    if (this.workspaceContext.getWorkspace().id === workspace.id) return;
    await this.workingCopyBackupTracker.flush();
    for (const group of this.editor.groups) {
      for (const input of [...group.inputs]) group.closeEditor(input);
    }
    await this.workingCopyBackupTracker.flush();
    this.workingCopyBackups.switchWorkspace(workspace.id);
    await this.storage.flush(WillSaveStateReason.WORKSPACE_CHANGE);
    this.storage.switchWorkspace(workspace.id);
    this.workbenchWindow.setWorkbenchState(
      workbenchStateFromWorkspaceIdentifier(workspace),
    );
    this.workspaceContext.updateWorkspace(workspace);
    await this.restoreWorkingCopyBackups(this.workingCopyBackups, this.editor);
  }
}

function requiredViewContainer(
  service: IViewDescriptorService,
  location: ViewContainerLocation,
  containerId: string,
) {
  const container = service
    .getViewContainers(location)
    .find((candidate) => candidate.id === containerId);
  if (!container) {
    throw new Error(
      `Workbench session references an unavailable ${location} view container: ${containerId}`,
    );
  }
  return container;
}

function appServerStatusEntry(state: AppServerConnectionState) {
  switch (state) {
    case "ready":
      return {
        text: "Ready",
        ariaLabel: "App Server ready",
      };
    case "stopped":
      return {
        text: "App Server unavailable",
        ariaLabel: "App Server unavailable",
        tooltip: "No App Server host is connected",
      };
    case "crashed":
      return {
        text: "App Server crashed",
        ariaLabel: "App Server crashed",
      };
    case "restarting":
      return {
        text: "App Server restarting\u2026",
        ariaLabel: "App Server restarting",
      };
    case "stopping":
      return {
        text: "App Server stopping\u2026",
        ariaLabel: "App Server stopping",
      };
    case "initializing":
      return {
        text: "App Server initializing\u2026",
        ariaLabel: "App Server initializing",
      };
    case "starting":
      return {
        text: "App Server starting\u2026",
        ariaLabel: "App Server starting",
      };
  }
}
