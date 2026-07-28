import { cloneDocumentStyles } from "../../base/browser/domStylesheets.js";
import {
  isRegisteredWindow,
  registerWindow,
} from "../../base/browser/window.js";
import {
  type IDisposable,
  DisposableOwner,
} from "../../base/common/lifecycle.js";
import { environment } from "../../base/common/platform.js";
import type {
  ProductConfiguration,
} from "../../product/common/product.js";
import type {
  AppServerConnectionState,
  ZetaRendererApi,
} from "../../platform/app-server/common/renderer-api.js";
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
import {
  IContextMenuService,
} from "../../platform/contextview/browser/contextMenu.js";
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
  bindColorTheme,
} from "../../platform/theme/browser/themeStyles.js";
import {
  IThemeService,
  ThemeService,
} from "../../platform/theme/common/themeService.js";
import {
  type IAnyWorkspaceIdentifier,
  type IWorkspace,
  IWorkspaceContextService,
} from "../../platform/workspace/common/workspace.js";
import { WorkbenchConfiguration } from "../common/configuration.js";
import {
  bindWorkbenchContextKeys,
  workbenchStateToContextValue,
} from "../common/contextkeys.js";
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
import { getWorkbenchColorTheme } from "../common/theme.js";
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
  IViewDescriptorService,
  ViewDescriptorService,
} from "../services/views/common/viewDescriptorService.js";
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
import {
  applyWorkbenchPartVisibilityContext,
  IWorkbenchLayoutService,
  WorkbenchLayout,
  type WorkbenchPartId,
} from "./layout.js";
import type { WorkbenchPart } from "./part.js";
import {
  AuxiliarybarPart,
} from "./parts/auxiliarybar/auxiliarybarPart.js";
import { EditorPart, IEditorPart } from "./parts/editor/editorPart.js";
import { SessionPart } from "./parts/session/sessionPart.js";
import { SidebarPart } from "./parts/sidebar/sidebarPart.js";
import { StatusbarPart } from "./parts/statusbar/statusbarPart.js";
import type {
  TitlebarPartFactory,
} from "./parts/titlebar/titlebarPart.js";
import {
  ViewPaneContainer,
} from "./parts/views/viewPaneContainer.js";
import { Viewlet } from "./parts/views/viewlet.js";
import { installWorkbenchStyles } from "./style.js";

/** Host-specific inputs required to construct a workbench. */
export interface IStartWorkbenchOptions {
  readonly product: ProductConfiguration;
  readonly api: ZetaRendererApi;
  readonly container: HTMLElement | null;
  readonly workspace: IAnyWorkspaceIdentifier;
  readonly configurationApi?: IConfigurationApi;
  readonly keybindingsResourceApi?: IKeybindingsResourceApi;
  readonly nativeHostApi?: INativeHostApi;
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
  createContextMenuService,
  createTitlebarPart,
}: IStartWorkbenchOptions): IDisposable {
  return new Workbench(
    product,
    api,
    container ?? document.body,
    workspace,
    configurationApi,
    keybindingsResourceApi,
    nativeHostApi,
    createContextMenuService,
    createTitlebarPart,
  );
}

/** Owns the renderer workbench, its parts, commands, and runtime layout. */
export class Workbench extends DisposableOwner {
  constructor(
    product: ProductConfiguration,
    api: ZetaRendererApi,
    workbenchRoot: HTMLElement,
    workspace: IAnyWorkspaceIdentifier,
    configurationApi: IConfigurationApi | undefined,
    keybindingsResourceApi: IKeybindingsResourceApi | undefined,
    nativeHostApi: INativeHostApi | undefined,
    createContextMenuService: WorkbenchContextMenuServiceFactory,
    createTitlebarPart: TitlebarPartFactory,
  ) {
    super();
    installWorkbenchStyles();
    const services = new ServiceCollection();
    const instantiationService = new InstantiationService(services);
    services.set(IRendererApiService, api);
    if (nativeHostApi) {
      services.set(INativeHostService, nativeHostApi);
    }
    const workspaceContext = new WorkspaceContextService(workspace);
    services.set(IWorkspaceContextService, workspaceContext);
    const currentWorkspace = workspaceContext.getWorkspace();
    const workbenchState = workspaceContext.getWorkbenchState();
    workbenchRoot.classList.add("zeta-workbench");
    workbenchRoot.setAttribute("data-product", product.id);
    workbenchRoot.setAttribute("data-runtime", environment.runtime);
    workbenchRoot.setAttribute("data-os", environment.os);
    workbenchRoot.setAttribute(
      "data-workbench-state",
      workbenchStateToContextValue(workbenchState),
    );
    this.defer(() => {
      workbenchRoot.classList.remove("zeta-workbench");
      workbenchRoot.removeAttribute("data-product");
      workbenchRoot.removeAttribute("data-runtime");
      workbenchRoot.removeAttribute("data-os");
      workbenchRoot.removeAttribute("data-workbench-state");
      workbenchRoot.replaceChildren();
    });

    const ownerDocument = workbenchRoot.ownerDocument;
    const targetWindow = ownerDocument.defaultView;
    if (targetWindow && !isRegisteredWindow(targetWindow)) {
      this.own(registerWindow(targetWindow));
    }
    if (ownerDocument !== document) {
      this.own(cloneDocumentStyles(document, ownerDocument));
    }

    const configuration = this.own(new WorkbenchConfigurationService({
      api: configurationApi,
    }));
    services.set(IConfigurationService, configuration);
    const themeService = this.own(new ThemeService(
      getWorkbenchColorTheme(
        configuration.getValue(WorkbenchConfiguration.colorTheme),
      ),
    ));
    services.set(IThemeService, themeService);
    this.own(configuration.onDidChangeConfiguration((event) => {
      if (event.affectsConfiguration(WorkbenchConfiguration.colorTheme)) {
        themeService.setColorTheme(getWorkbenchColorTheme(
          configuration.getValue(WorkbenchConfiguration.colorTheme),
        ));
      }
    }));
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
    applyWorkbenchPartVisibilityContext(contextKeys, "sidebar", true);
    applyWorkbenchPartVisibilityContext(contextKeys, "auxiliarybar", true);
    applyWorkbenchPartVisibilityContext(contextKeys, "editor", true);
    const viewDescriptors = this.own(new ViewDescriptorService({
      contextKeyService: contextKeys,
    }));
    services.set(IViewDescriptorService, viewDescriptors);
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
    const quickInput = this.own(new WorkbenchQuickInputService({
      container: workbenchRoot,
      contextKeyService: contextKeys,
    }));
    services.set(IQuickInputService, quickInput);
    const contextMenus = this.own(createContextMenuService({
      menuService: menus,
      keybindingService: keybindings,
      ownerDocument,
    }));
    services.set(IContextMenuService, contextMenus);
    const contributions = this.own(
      WorkbenchContributionsRegistry.createHost(services),
    );
    contributions.advance(WorkbenchPhase.BlockStartup);

    const titlebar = this.own(createTitlebarPart({
      menuService: menus,
      contextMenuService: contextMenus,
      ownerDocument,
      title: workspaceTitle(currentWorkspace, product.name),
    }));
    const sidebar = this.own(new SidebarPart(ownerDocument));
    const sidebarViewContainer = requiredViewContainer(
      viewDescriptors,
      ViewContainerLocation.Sidebar,
    );
    sidebar.setViewlet(new Viewlet({
      viewContainer: sidebarViewContainer,
      model: viewDescriptors.getViewContainerModel(
        sidebarViewContainer.id,
      ),
      instantiationService,
      contextKeyService: contextKeys,
      ownerDocument,
    }));
    const session = this.own(new SessionPart(ownerDocument));
    const editor = this.own(new EditorPart(ownerDocument, {
      keybindingService: keybindings,
    }));
    services.set(IEditorPart, editor);
    const auxiliarybar = this.own(new AuxiliarybarPart(ownerDocument));
    const auxiliaryViewContainer = requiredViewContainer(
      viewDescriptors,
      ViewContainerLocation.AuxiliaryBar,
    );
    auxiliarybar.setViewPaneContainer(new ViewPaneContainer({
      viewContainer: auxiliaryViewContainer,
      model: viewDescriptors.getViewContainerModel(
        auxiliaryViewContainer.id,
      ),
      instantiationService,
      contextKeyService: contextKeys,
      ownerDocument,
    }));
    const statusbar = this.own(new StatusbarPart(
      statusbarService,
      ownerDocument,
    ));

    const parts = new Map<WorkbenchPartId, WorkbenchPart>([
      ["titlebar", titlebar],
      ["statusbar", statusbar],
      ["sidebar", sidebar],
      ["session", session],
      ["auxiliarybar", auxiliarybar],
      ["editor", editor],
    ]);
    const layout = this.own(
      new WorkbenchLayout(workbenchRoot, parts),
    );
    services.set(IWorkbenchLayoutService, layout);
    this.own(layout.onDidChangePartVisibility(({ partId, visible }) => {
      applyWorkbenchPartVisibilityContext(contextKeys, partId, visible);
    }));
    contributions.advance(WorkbenchPhase.BlockRestore);
    layout.layout();
    contributions.advance(WorkbenchPhase.AfterRestored);
    const eventuallyTimer = globalThis.setTimeout(
      () => contributions.advance(WorkbenchPhase.Eventually),
      2_000,
    );
    this.defer(() => globalThis.clearTimeout(eventuallyTimer));
  }
}

function workspaceTitle(
  workspace: IWorkspace,
  productName: string,
): string {
  const name = workspace.name ?? workspace.folders[0]?.name;
  return name ? `${name} — ${productName}` : productName;
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
