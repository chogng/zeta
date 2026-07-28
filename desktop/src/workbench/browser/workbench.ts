import { Button } from "../../base/browser/ui/index.js";
import { cloneDocumentStyles } from "../../base/browser/domStylesheets.js";
import {
  isRegisteredWindow,
  registerWindow,
} from "../../base/browser/window.js";
import {
  type IDisposable,
  DisposableOwner,
} from "../../base/common/lifecycle.js";
import { LxIcon } from "../../base/common/lxicons.js";
import { environment } from "../../base/common/platform.js";
import type { ZetaRendererApi } from "../../platform/app-server/common/renderer-api.js";
import {
  MenuService,
} from "../../platform/actions/common/menuService.js";
import {
  ICommandService,
  CommandService,
} from "../../platform/commands/common/command-registry.js";
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
  DialogService,
  IDialogService,
} from "../../platform/dialogs/common/dialogs.js";
import {
  bindColorTheme,
} from "../../platform/theme/browser/themeStyles.js";
import {
  darkColorTheme,
} from "../../platform/theme/common/colorTheme.js";
import {
  IThemeService,
  ThemeService,
} from "../../platform/theme/common/themeService.js";
import {
  IWorkspaceContextService,
  type IWorkspaceContext,
  WorkbenchState,
} from "../../platform/workspace/common/workspace.js";
import { IRendererApiService } from "../common/services.js";
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
  KeybindingsResourceContribution,
} from "../services/keybinding/browser/keybindingsResourceContribution.js";
import {
  WorkbenchKeybindingsResourceService,
} from "../services/keybinding/browser/keybindingsResourceService.js";
import {
  WorkspaceContextService,
} from "../services/workspaces/browser/workspaceContextService.js";
import {
  WorkbenchConfigurationService,
} from "../services/configuration/browser/configurationService.js";
import type {
  WorkbenchContextMenuServiceFactory,
} from "../services/contextmenu/common/contextMenuService.js";
import { WorkbenchLayout, type SerializableGrid, type WorkbenchPartId } from "./layout.js";
import type { WorkbenchPart } from "./part.js";
import {
  AuxiliarybarPart,
  EditorPart,
  SessionPart,
  SidebarPart,
  StatusbarPart,
  type TitlebarPartFactory,
  ViewPaneContainer,
  Viewlet,
} from "./parts/index.js";
import { installWorkbenchStyles } from "./style.js";
import "./workbench.contribution.js";

/** Host-specific inputs required to construct a workbench. */
export interface IStartWorkbenchOptions {
  readonly api: ZetaRendererApi;
  readonly container: HTMLElement | null;
  readonly workspace: IWorkspaceContext;
  readonly configurationApi?: IConfigurationApi;
  readonly keybindingsResourceApi?: IKeybindingsResourceApi;
  readonly createContextMenuService: WorkbenchContextMenuServiceFactory;
  readonly createTitlebarPart: TitlebarPartFactory;
}

/** Starts the browser workbench and binds its commands to the initial UI. */
export function startWorkbench({
  api,
  container,
  workspace,
  configurationApi,
  keybindingsResourceApi,
  createContextMenuService,
  createTitlebarPart,
}: IStartWorkbenchOptions): IDisposable {
  return new Workbench(
    api,
    container ?? document.body,
    workspace,
    configurationApi,
    keybindingsResourceApi,
    createContextMenuService,
    createTitlebarPart,
  );
}

/** Owns the renderer workbench, its parts, commands, and runtime layout. */
export class Workbench extends DisposableOwner {
  constructor(
    api: ZetaRendererApi,
    workbenchRoot: HTMLElement,
    workspace: IWorkspaceContext,
    configurationApi: IConfigurationApi | undefined,
    keybindingsResourceApi: IKeybindingsResourceApi | undefined,
    createContextMenuService: WorkbenchContextMenuServiceFactory,
    createTitlebarPart: TitlebarPartFactory,
  ) {
    super();
    installWorkbenchStyles();
    workbenchRoot.classList.add("zeta-workbench");
    workbenchRoot.setAttribute("data-runtime", environment.runtime);
    workbenchRoot.setAttribute("data-os", environment.os);
    workbenchRoot.setAttribute(
      "data-workbench-state",
      workbenchStateName(workspace.state),
    );
    this.defer(() => {
      workbenchRoot.classList.remove("zeta-workbench");
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

    const services = new ServiceCollection();
    services.set(IRendererApiService, api);
    const workspaceContext = new WorkspaceContextService(workspace);
    services.set(IWorkspaceContextService, workspaceContext);
    const configuration = this.own(new WorkbenchConfigurationService({
      api: configurationApi,
    }));
    services.set(IConfigurationService, configuration);
    const themeService = this.own(new ThemeService(darkColorTheme));
    services.set(IThemeService, themeService);
    this.own(bindColorTheme(themeService, workbenchRoot));
    const statusbarService = this.own(new StatusbarService());
    services.set(IStatusbarService, statusbarService);
    this.own(statusbarService.addEntry(
      {
        text: "Ready",
        ariaLabel: "Application ready",
      },
      {
        id: "zeta.status.ready",
        alignment: StatusbarAlignment.Left,
      },
    ));
    const dialogService = this.own(new DialogService(
      new BrowserDialogHandler(workbenchRoot),
    ));
    services.set(IDialogService, dialogService);
    const commands = new CommandService(services);
    services.set(ICommandService, commands);
    const contextKeys = this.own(new ContextKeyService());
    services.set(IContextKeyService, contextKeys);
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
    this.own(new KeybindingsResourceContribution({
      service: keybindingsResource,
    }));
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
    const contextMenus = this.own(createContextMenuService({
      menuService: menus,
      keybindingService: keybindings,
      ownerDocument,
    }));
    services.set(IContextMenuService, contextMenus);

    const titlebar = this.own(createTitlebarPart({
      menuService: menus,
      contextMenuService: contextMenus,
      ownerDocument,
      title: workspaceTitle(workspace),
    }));
    const sidebar = this.own(new SidebarPart(ownerDocument));
    sidebar.setViewlet(new Viewlet(
      "zeta.sidebar",
      "Navigation",
      ownerDocument,
    ));
    const session = this.own(new SessionPart(ownerDocument));
    const editor = this.own(new EditorPart(ownerDocument));
    editor.setView(new Button({
      label: "Start conversation",
      ownerDocument,
      icon: LxIcon.start,
      onClick: () => commands.executeCommand("zeta.startTurn"),
    }));
    const auxiliarybar = this.own(new AuxiliarybarPart(ownerDocument));
    auxiliarybar.setViewPaneContainer(
      new ViewPaneContainer("zeta.auxiliary", ownerDocument),
    );
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
      new WorkbenchLayout(workbenchRoot, parts, defaultWorkbenchGrid),
    );
    layout.layout();
  }
}

const defaultWorkbenchGrid: SerializableGrid = {
  type: "split",
  orientation: "vertical",
  children: [
    { type: "part", partId: "titlebar", size: "34px" },
    {
      type: "split",
      orientation: "horizontal",
      size: "1fr",
      children: [
        { type: "part", partId: "sidebar", size: "220px" },
        {
          type: "split",
          orientation: "vertical",
          size: "1fr",
          children: [
            { type: "part", partId: "session", size: "28px" },
            { type: "part", partId: "editor", size: "1fr" },
          ],
        },
        { type: "part", partId: "auxiliarybar", size: "220px" },
      ],
    },
    { type: "part", partId: "statusbar", size: "22px" },
  ],
};

function workspaceTitle(workspace: IWorkspaceContext): string {
  return workspace.state === WorkbenchState.EMPTY
    ? "Zeta"
    : `${workspace.label} — Zeta`;
}

function workbenchStateName(state: WorkbenchState): string {
  switch (state) {
    case WorkbenchState.EMPTY:
      return "empty";
    case WorkbenchState.FOLDER:
      return "folder";
    case WorkbenchState.WORKSPACE:
      return "workspace";
  }
}
