import "./style.js";
import { setHoverDelegate } from "../../base/browser/ui/hover/hoverDelegate.js";
import {
  type IDisposable,
  DisposableOwner,
} from "../../base/common/lifecycle.js";
import { assertDefined } from "../../base/common/types.js";
import type {
  ProductConfiguration,
} from "../../product/common/product.js";
import type { AppServerConnectionState } from "../../platform/app-server/common/appServerApi.js";
import type { IRendererHost } from "../../platform/renderer/common/rendererHost.js";
import type {
  INativeHostApi,
} from "../../platform/native/common/nativeHost.js";
import {
  IMenuService,
  MenuService,
} from "../../platform/actions/common/menuService.js";
import {
  ICommandService,
} from "../../platform/commands/common/commands.js";
import {
  IQuickInputService,
} from "../../platform/quickinput/common/quickInput.js";
import {
  IContextKeyService,
  ContextKeyService,
} from "../../platform/contextkey/common/contextkey.js";
import {
  type IConfigurationApi,
  IConfigurationService,
} from "../../platform/configuration/common/configuration.js";
import { IStorageService, WillSaveStateReason } from "../../platform/storage/common/storage.js";
import {
  IContextMenuService,
} from "../../platform/contextview/browser/contextMenu.js";
import {
  IContextViewService,
} from "../../platform/contextview/browser/contextView.js";
import {
  BrowserContextViewService,
} from "../../platform/contextview/browser/contextViewService.js";
import { HoverService } from "../../platform/hover/browser/hoverService.js";
import { IHoverService } from "../../platform/hover/common/hoverService.js";
import {
  InstantiationService,
  ServiceCollection,
} from "../../platform/instantiation/common/instantiation.js";
import {
  IKeybindingService,
} from "../../platform/keybinding/common/keybinding.js";
import {
  type IKeybindingsResourceApi,
  IKeybindingsResourceService,
} from "../../platform/keybinding/common/keybindingsResource.js";
import {
  IKeyboardLayoutService,
} from "../../platform/keyboardLayout/common/keyboardLayout.js";
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
  WorkbenchContributionsRegistry,
  WorkbenchPhase,
} from "../common/contributions.js";
import {
  IDialogsModel,
  IWorkbenchDialogHandler,
} from "../common/dialogs.js";
import {
  INativeHostService,
  IRendererApiService,
} from "../common/services.js";
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
  BrowserKeyboardLayoutService,
} from "../services/keybinding/browser/keyboardLayoutService.js";
import {
  WorkbenchKeybindingService,
} from "../services/keybinding/browser/keybindingService.js";
import {
  WorkbenchKeybindingsResourceService,
} from "../services/keybinding/browser/keybindingsResourceService.js";
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
import {
  IWorkbenchSessionService,
  WorkbenchSessionService,
} from "../services/sessions/common/sessionService.js";
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
  CommandService,
} from "../services/commands/common/commandService.js";
import {
  WorkbenchQuickInputService,
} from "../services/quickinput/browser/quickInputService.js";
import { ISettingsService } from "../services/preferences/common/settings.js";
import { SettingsService } from "../services/preferences/common/settingsService.js";
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
import {
  ViewPaneContainer,
} from "./parts/views/viewPaneContainer.js";
import { PaneComposite } from "./parts/views/paneComposite.js";
import { IWorkbenchWindowService, WorkbenchWindow } from "./window.js";
import { TerminalProcessService } from "../../platform/terminal/browser/terminalProcessService.js";
import { TerminalService } from "../services/terminal/browser/terminalService.js";
import { ITerminalService } from "../services/terminal/common/terminal.js";
import { ITextFileService, TextFileService } from "../services/textfile/common/textFileService.js";

/** Host-specific inputs required to construct a workbench. */
export interface IStartWorkbenchOptions {
  readonly product: ProductConfiguration;
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
  private readonly workspaceContext: WorkspaceContextService;
  private readonly storage: BrowserStorageService;
  private readonly editor: EditorPart;
  private readonly workbenchWindow: WorkbenchWindow;
  private workspaceSwitchQueue: Promise<void> = Promise.resolve();

  constructor(
    product: ProductConfiguration,
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
    const services = new ServiceCollection();
    const instantiationService = new InstantiationService(services);
    services.set(IRendererApiService, api);
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
      workspaceContextService: workspaceContext,
    });
    services.set(IFileService, fileService);
    const textFileService = new TextFileService(fileService);
    services.set(ITextFileService, textFileService);
    services.set(
      IWorkspaceSearchService,
      new BrowserWorkspaceSearchService(api.workspaceSearch),
    );
    const terminalService = this.own(new TerminalService(new TerminalProcessService(api)));
    services.set(ITerminalService, terminalService);
    const workbenchState = workspaceContext.getWorkbenchState();
    const workbenchWindow = this.own(new WorkbenchWindow({
      root: workbenchRoot,
      productId: product.id,
      workbenchState,
    }));
    services.set(IWorkbenchWindowService, workbenchWindow);
    const ownerDocument = workbenchWindow.ownerDocument;

    const configuration = this.own(new WorkbenchConfigurationService({
      api: configurationApi,
    }));
    services.set(IConfigurationService, configuration);
    const ownerWindow = ownerDocument.defaultView;
    if (!ownerWindow) {
      throw new Error("Workbench requires an owner window");
    }
    const storage = this.own(new BrowserStorageService({
      ownerWindow,
      applicationId: product.id,
      workspaceId: workspace.id,
    }));
    this.workbenchWindow = workbenchWindow;
    this.storage = storage;
    services.set(IStorageService, storage);
    const themeService = this.own(new ThemeService(
      resolveWorkbenchColorTheme(
        configuration.getValue(WorkbenchConfiguration.colorTheme),
        ownerWindow.matchMedia("(prefers-color-scheme: dark)").matches,
      ),
    ));
    services.set(IThemeService, themeService);
    services.set(IUserThemeService, userThemeService ?? UnavailableUserThemeService);
    services.set(
      IFileIconThemeService,
      this.own(new SetiFileIconThemeService(themeService)),
    );
    this.own(new WorkbenchThemeController(
      configuration,
      themeService,
      ownerWindow,
    ));
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
    const connectionStateSubscription = api.appServer.onConnectionState(
      (state) => connectionStatus.update(appServerStatusEntry(state)),
    );
    this.defer(() => connectionStateSubscription.dispose());
    void Promise.resolve()
      .then(() => api.appServer.getConnectionState())
      .then((state) => connectionStatus.update(appServerStatusEntry(state)))
      .catch((error: unknown) => {
        console.error("Failed to read App Server connection state", error);
        connectionStatus.update(appServerStatusEntry("crashed"));
      });
    const dialogService = this.own(new DialogService());
    services.set(IDialogService, dialogService);
    services.set(IDialogsModel, dialogService.model);
    services.set(
      IWorkbenchDialogHandler,
      new BrowserDialogHandler(workbenchRoot),
    );
    const commands = this.own(new CommandService(services));
    services.set(ICommandService, commands);
    const contextKeys = this.own(new ContextKeyService());
    services.set(IContextKeyService, contextKeys);
    this.own(bindWorkbenchContextKeys(contextKeys, workspaceContext));
    const viewDescriptors = this.own(new ViewDescriptorService({
      contextKeyService: contextKeys,
    }));
    services.set(IViewDescriptorService, viewDescriptors);
    const sessionService = this.own(new WorkbenchSessionService(api));
    services.set(IWorkbenchSessionService, sessionService);
    const keyboardLayout = this.own(new BrowserKeyboardLayoutService({
      navigator: ownerDocument.defaultView?.navigator ?? navigator,
    }));
    services.set(IKeyboardLayoutService, keyboardLayout);
    const keybindingsResource = this.own(
      new WorkbenchKeybindingsResourceService({
        api: keybindingsResourceApi,
      }),
    );
    services.set(IKeybindingsResourceService, keybindingsResource);
    const keybindings = this.own(new WorkbenchKeybindingService({
      ownerDocument,
      commandService: commands,
      contextKeyService: contextKeys,
      keyboardLayoutService: keyboardLayout,
      statusbarService,
    }));
    services.set(IKeybindingService, keybindings);
    void configuration.reload().catch((error: unknown) => {
      console.error("Failed to initialize configuration", error);
    });
    void keybindingsResource.reload().catch((error: unknown) => {
      console.error("Failed to initialize keybindings resource", error);
    });
    const menus = new MenuService(commands, contextKeys);
    services.set(IMenuService, menus);
    const contextViews = this.own(
      new BrowserContextViewService(workbenchRoot),
    );
    services.set(IContextViewService, contextViews);
    const quickInput = this.own(new WorkbenchQuickInputService({
      container: workbenchRoot,
      contextKeyService: contextKeys,
    }));
    services.set(IQuickInputService, quickInput);
    const settings = this.own(new SettingsService());
    services.set(ISettingsService, settings);
    const contextMenus = this.own(createContextMenuService({
      menuService: menus,
      keybindingService: keybindings,
      contextViewService: contextViews,
    }));
    services.set(IContextMenuService, contextMenus);
    const hoverService = this.own(new HoverService(
      configuration,
      contextViews,
      contextMenus,
    ));
    services.set(IHoverService, hoverService);
    this.own(setHoverDelegate(hoverService));
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
    const editor = this.own(new EditorPart(ownerDocument, {
      configurationService: configuration,
      keybindingService: keybindings,
      textFileService,
      titleActions: {
        menuService: menus,
        contextMenuProvider: contextMenus,
      },
    }));
    services.set(IEditorPart, editor);
    const sidebarCompositeDescriptor = requiredViewContainer(
      viewDescriptors,
      ViewContainerLocation.Sidebar,
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
    const panel = this.own(new PanelPart({
      ownerDocument,
      viewDescriptorService: viewDescriptors,
      contextMenuProvider: contextMenus,
    }));
    this.editor = editor;
    const panelCompositeDescriptor = requiredViewContainer(
      viewDescriptors,
      ViewContainerLocation.Panel,
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
    const auxiliarybar = this.own(new AuxiliarybarPart(ownerDocument));
    const auxiliaryViewContainer = requiredViewContainer(
      viewDescriptors,
      ViewContainerLocation.AuxiliaryBar,
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
      ["editor", editor],
      ["panel", panel],
    ]);
    const layout = this.own(new WorkbenchLayout(workbenchRoot, parts, {
      storageService: storage,
    }));
    services.set(IWorkbenchLayoutService, layout);
    // Fixed Panel and Auxiliary Bar views may depend on the host layout during construction.
    openPanelComposite(panelCompositeDescriptor.id);
    const auxiliaryPaneContainer = new ViewPaneContainer({
      viewContainer: auxiliaryViewContainer,
      model: viewDescriptors.getViewContainerModel(auxiliaryViewContainer.id),
      instantiationService,
      contextKeyService: contextKeys,
      ownerDocument,
    });
    auxiliarybar.setViewPaneContainer(auxiliaryPaneContainer);
    services.set(IViewsService, new ViewsService({
      viewDescriptorService: viewDescriptors,
      openViewContainer: (container) => {
        switch (container.location) {
          case ViewContainerLocation.Sidebar:
            layout.showPart("sidebar");
            return openSidebarComposite(container.id);
          case ViewContainerLocation.AuxiliaryBar:
            layout.showPart("auxiliarybar");
            return container.id === auxiliaryViewContainer.id
              ? auxiliaryPaneContainer
              : undefined;
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
    this.own(panel.onDidSelectComposite(
      ({ compositeId }) => {
        if (panel.activeCompositeId === compositeId) return;
        openPanelComposite(compositeId);
      },
    ));
    void sessionService.initialize();
    contributions.advance(WorkbenchPhase.BlockRestore);
    layout.layout();
    contributions.advance(WorkbenchPhase.AfterRestored);
    const eventuallyTimer = globalThis.setTimeout(
      () => contributions.advance(WorkbenchPhase.Eventually),
      2_000,
    );
    this.defer(() => globalThis.clearTimeout(eventuallyTimer));
    this.defer(() => {
      void storage.flush(WillSaveStateReason.SHUTDOWN);
    });
  }

  /** Applies a host-authoritative workspace replacement without rebuilding the Workbench. */
  updateWorkspace(workspace: IAnyWorkspaceIdentifier): Promise<void> {
    const switching = this.workspaceSwitchQueue.then(() => this.doUpdateWorkspace(workspace));
    this.workspaceSwitchQueue = switching.then(() => undefined, () => undefined);
    return switching;
  }

  private async doUpdateWorkspace(workspace: IAnyWorkspaceIdentifier): Promise<void> {
    if (this.workspaceContext.getWorkspace().id === workspace.id) return;
    for (const group of this.editor.groups) {
      for (const input of [...group.inputs]) group.closeEditor(input);
    }
    await this.storage.flush(WillSaveStateReason.WORKSPACE_CHANGE);
    this.storage.switchWorkspace(workspace.id);
    this.workbenchWindow.setWorkbenchState(
      workbenchStateFromWorkspaceIdentifier(workspace),
    );
    this.workspaceContext.updateWorkspace(workspace);
  }
}

function requiredViewContainer(
  service: IViewDescriptorService,
  location: ViewContainerLocation,
) {
  const container = service.getDefaultViewContainer(location);
  if (!container) {
    throw new Error(
      `No default view container is registered for ${location}`,
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
