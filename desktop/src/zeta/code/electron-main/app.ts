import {
  app,
  BrowserWindow,
  dialog,
  ipcMain,
  Menu,
  screen,
  type Event as ElectronEvent,
} from "electron/main";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { APP_SERVER_SCHEMA_HASH } from "../../../../generated/app-server/types.js";
import {
  DisposableOwner,
  DisposableStore,
  type IDisposable,
} from "../../base/common/lifecycle.js";
import { assertDefined } from "../../base/common/types.js";
import {
  DisposableTracker,
  installDisposableTracker,
} from "../../base/common/disposableTracker.js";
import type {
  ProductConfiguration,
} from "../../product/common/product.js";
import {
  ElectronContextMenu,
} from "../../base/parts/contextmenu/electron-main/contextmenu.js";
import {
  appServerIpcRoutes,
} from "../../platform/app-server/electron-main/app-server-ipc.js";
import { buildAppServerEnvironment } from "../../platform/app-server/common/appServerEnvironment.js";
import {
  AppServerSupervisor,
} from "../../platform/app-server/electron-main/app-server-supervisor.js";
import { appServerExecutablePath } from "../../platform/app-server/electron-main/app-server-package.js";
import { normalizeEntryUrl, TrustedIpcRouter } from "../../platform/ipc/electron-main/trustedIpcRouter.js";
import {
  BROWSER_VIEW_EVENT_CHANNEL,
} from "../../platform/browser/common/browserView.js";
import {
  browserViewIpcRoutes,
} from "../../platform/browser/electron-main/browserViewIpc.js";
import {
  BrowserViewMainService,
} from "../../platform/browser/electron-main/browserViewMainService.js";
import {
  CONFIGURATION_CHANGED_CHANNEL,
} from "../../platform/configuration/common/configuration.js";
import {
  ConfigurationMainService,
  configurationIpcRoutes,
} from "../../platform/configuration/electron-main/configurationMainService.js";
import {
  nativeContextMenuIpcRoutes,
} from "../../platform/contextview/electron-main/contextMenuIpc.js";
import { fileIpcRoutes } from "../../platform/files/electron-main/fileIpcRoutes.js";
import { extensionIpcRoutes } from "../../platform/extensions/electron-main/extensionIpcRoutes.js";
import { diffIpcRoutes } from "../../platform/diff/electron-main/diffIpcRoutes.js";
import { documentCollaborationIpcRoutes } from "../../platform/collaboration/electron-main/documentCollaborationIpcRoutes.js";
import { syntaxIpcRoutes } from "../../platform/syntax/electron-main/syntaxIpcRoutes.js";
import { gitIpcRoutes } from "../../platform/git/electron-main/gitIpcRoutes.js";
import {
  KEYBINDINGS_RESOURCE_CHANGED_CHANNEL,
} from "../../platform/keybinding/common/keybindingsResource.js";
import {
  KeybindingsResourceMainService,
  keybindingsResourceIpcRoutes,
} from "../../platform/keybinding/electron-main/keybindingsResourceMainService.js";
import {
  migrateLegacyKeybindings,
} from "../../platform/keybinding/electron-main/migrateLegacyKeybindings.js";
import {
  NativeMenubarMainService,
  nativeMenubarIpcRoutes,
} from "../../platform/menubar/electron-main/menubarMainService.js";
import {
  nativeHostIpcRoutes,
} from "../../platform/native/electron-main/nativeHostIpc.js";
import { NATIVE_HOST_ACCESSIBILITY_SUPPORT_CHANGED_CHANNEL } from "../../platform/native/common/nativeHost.js";
import { searchIpcRoutes } from "../../platform/search/electron-main/searchIpcRoutes.js";
import { sessionIpcRoutes } from "../../platform/sessions/electron-main/sessionIpcRoutes.js";
import { sessionsWindowIpcRoutes } from "../../sessions/electron-main/sessionsWindowIpc.js";
import {
  StateService,
} from "../../platform/state/node/stateService.js";
import { terminalIpcRoutes } from "../../platform/terminal/electron-main/terminalIpcRoutes.js";
import { userThemeIpcRoutes } from "../../platform/theme/electron-main/userThemeIpc.js";
import { UserThemeFileService } from "../../platform/theme/node/userThemeFileService.js";
import { typstIpcRoutes } from "../../platform/typst/electron-main/typstIpcRoutes.js";
import {
  applyWindowState,
  resolveBrowserWindowOptions,
} from "../../platform/windows/electron-main/windows.js";
import { WindowMode } from "../../platform/window/electron-main/window.js";
import {
  WindowsStateHandler,
} from "../../platform/windows/electron-main/windowsStateHandler.js";
import { type IAnyWorkspaceIdentifier, isSingleFolderWorkspaceIdentifier, serializeWorkspaceIdentifier, UNKNOWN_EMPTY_WINDOW_WORKSPACE } from "../../platform/workspace/common/workspace.js";
import { WORKSPACE_CONTEXT_CHANGED_CHANNEL } from "../../platform/workspace/common/workspaceIpc.js";
import { createAppServerWorkspaceTransitionAdapter } from "../../platform/workspaces/electron-main/appServerWorkspaceTransition.js";
import { type IWorkspaceTransitionFailure, type WorkspaceTransitionMainServiceOptions, WorkspaceTransitionFailureKind, WorkspaceTransitionMainService, WorkspaceTransitionStatus } from "../../platform/workspaces/electron-main/workspaceTransitionMainService.js";
import { WorkspaceContextMainService, WorkspacesMainService, workspaceContextIpcRoutes } from "../../platform/workspaces/electron-main/workspacesMainService.js";
export type AppServerStartupMode = "required" | "disabled";

export interface ZetaApplicationOptions {
  readonly product: ProductConfiguration;
  readonly rendererRoot: string;
  /** Selects whether this Electron process starts the App Server before opening its window. */
  readonly appServerStartupMode: AppServerStartupMode;
}

interface PersistentServices {
  readonly state: StateService;
  readonly configuration: ConfigurationMainService;
  readonly keybindings: KeybindingsResourceMainService;
}

interface RendererEntry {
  readonly file: string;
  readonly url: string;
  readonly useDevelopmentUrl: boolean;
}

/**
 * Owns the Electron application's services, primary window, IPC, and shutdown.
 */
export class ZetaApplication extends DisposableOwner {
  private readonly product: ProductConfiguration;
  private readonly rendererRoot: string;
  private readonly appServerStartupMode: AppServerStartupMode;
  private readonly disposableTracker: DisposableTracker | undefined;
  private readonly tracking: Disposable | undefined;
  private readonly trustedIpcRouter: TrustedIpcRouter;

  private supervisor: AppServerSupervisor | undefined;
  private mainWindow: BrowserWindow | undefined;
  private sessionsWindow: BrowserWindow | undefined;
  private persistentServices: PersistentServices | undefined;
  private _windowsStateHandler: WindowsStateHandler | undefined;
  private windowStateTracking: IDisposable | undefined;
  private closePersistentServicesPromise: Promise<void> | undefined;
  private quitRequested = false;
  private quitAfterStateSaved = false;
  private quitSaveStarted = false;

  private constructor(
    options: ZetaApplicationOptions,
    disposableTracker: DisposableTracker | undefined,
    tracking: Disposable | undefined,
  ) {
    super();
    this.product = options.product;
    this.rendererRoot = options.rendererRoot;
    this.appServerStartupMode = options.appServerStartupMode;
    this.disposableTracker = disposableTracker;
    this.tracking = tracking;
    this.trustedIpcRouter = this.own(new TrustedIpcRouter(ipcMain));

    app.on("before-quit", this.onBeforeQuit);
    app.on("will-quit", this.onWillQuit);
    app.on("accessibility-support-changed", this.onAccessibilitySupportChanged);
    this.defer(() => {
      app.removeListener("before-quit", this.onBeforeQuit);
      app.removeListener("will-quit", this.onWillQuit);
      app.removeListener("accessibility-support-changed", this.onAccessibilitySupportChanged);
    });
  }

  static create(options: ZetaApplicationOptions): ZetaApplication {
    const disposableTracker = app.isPackaged
      ? undefined
      : new DisposableTracker();
    const tracking = disposableTracker
      ? installDisposableTracker(disposableTracker)
      : undefined;
    return new ZetaApplication(options, disposableTracker, tracking);
  }

  async startupAfterReady(): Promise<void> {
    if (!app.isReady()) {
      throw new Error("Zeta application startup requires Electron to be ready");
    }
    if (process.platform !== "darwin") {
      Menu.setApplicationMenu(null);
    }

    await this.createPersistentServices();
    const workspaces = new WorkspacesMainService();
    const workspace = await this.resolveWorkspace(workspaces);
    this._windowsStateHandler = this.createWindowsStateHandler(workspace);
    const workspaceContext = this.own(new WorkspaceContextMainService(workspace));

    const supervisor = this.own(this.createAppServerSupervisor(workspace));
    this.supervisor = supervisor;
    if (this.appServerStartupMode === "required") {
      const appServerReady = await this.startAppServerWithRecovery(supervisor);
      if (!appServerReady) {
        return;
      }
    }
    await this.openFirstWindow(workspaceContext, workspaces, supervisor);
  }

  async disposeAfterStartupFailure(): Promise<void> {
    this.supervisor?.dispose();
    try {
      await this.closePersistentServices();
    } finally {
      this.dispose();
      this.releaseDisposableTracker();
    }
  }

  /** Brings this product instance to the foreground for a second-instance launch. */
  focusMainWindow(): void {
    const window = this.mainWindow;
    if (!window || window.isDestroyed()) return;
    if (window.isMinimized()) window.restore();
    window.focus();
  }

  private async createPersistentServices(): Promise<void> {
    const state = await StateService.create(
      join(app.getPath("userData"), "state.json"),
    );
    let configuration: ConfigurationMainService | undefined;
    let keybindings: KeybindingsResourceMainService | undefined;
    try {
      configuration = await ConfigurationMainService.create({
        filePath: join(app.getPath("userData"), "configuration.json"),
        onError: (error) => {
          console.error("Failed to process configuration", error);
        },
      });
      keybindings = await KeybindingsResourceMainService.create({
        filePath: join(app.getPath("userData"), "keybindings.json"),
        onError: (error) => {
          console.error("Failed to process keybindings resource", error);
        },
      });
      await migrateLegacyKeybindings(configuration, keybindings);
      this.persistentServices = { state, configuration, keybindings };
    } catch (error) {
      await Promise.all([
        state.close(),
        configuration?.close(),
        keybindings?.close(),
      ]);
      throw error;
    }
  }

  private async resolveWorkspace(
    workspaces: WorkspacesMainService,
  ): Promise<IAnyWorkspaceIdentifier> {
    try {
      return await workspaces.resolveStartupWorkspace({
        arguments: process.argv
          .slice(app.isPackaged ? 1 : 2)
          .filter((argument) => argument !== app.getAppPath()),
        cwd: process.cwd(),
      });
    } catch (error) {
      console.error("Failed to resolve startup workspace", error);
      return UNKNOWN_EMPTY_WINDOW_WORKSPACE;
    }
  }

  private createAppServerSupervisor(
    workspace: IAnyWorkspaceIdentifier,
  ): AppServerSupervisor {
    const executable = appServerExecutablePath({
      appPath: app.getAppPath(),
      isPackaged: app.isPackaged,
      platform: process.platform,
      resourcesPath: process.resourcesPath,
    });
    return new AppServerSupervisor({
      executable,
      args: ["app-server", "--listen", "stdio://"],
      environment: this.appServerEnvironment(workspace),
      session: {
        clientName: "zeta-desktop",
        clientVersion: app.getVersion(),
        schemaHash: APP_SERVER_SCHEMA_HASH,
        initializeTimeoutMs: 10_000,
        expectedServerName: "zeta-app-server",
      },
    });
  }

  private async startAppServerWithRecovery(
    supervisor: AppServerSupervisor,
  ): Promise<boolean> {
    while (!this.quitRequested) {
      try {
        await supervisor.start();
        return true;
      } catch (error) {
        console.error("App Server failed the startup gate", error);
        if (this.quitRequested) {
          return false;
        }

        const message = error instanceof Error
          ? error.message
          : "The App Server failed to start";
        const diagnostics = supervisor.diagnostics().trim();
        const detail = diagnostics
          ? `${message}\n\nDiagnostics:\n${diagnostics}`.slice(0, 8_000)
          : message;
        const result = await dialog.showMessageBox({
          type: "error",
          title: `${this.product.name} startup failed`,
          message: "The App Server could not be validated.",
          detail,
          buttons: ["Retry", "Quit"],
          defaultId: 0,
          cancelId: 1,
          noLink: true,
        });
        if (this.quitRequested || result.response !== 0) {
          if (!this.quitRequested) {
            app.quit();
          }
          return false;
        }
        await supervisor.stop();
      }
    }
    return false;
  }

  private async openFirstWindow(
    workspaceContext: WorkspaceContextMainService,
    workspaces: WorkspacesMainService,
    supervisor: AppServerSupervisor,
  ): Promise<void> {
    const windowsStateHandler = this.windowsStateHandler;
    const windowState = windowsStateHandler.restoreWindowState();
    const browserWindowOptions = resolveBrowserWindowOptions({
      state: windowState,
      webPreferences: this.createSandboxWebPreferences(),
    });
    const window = new BrowserWindow({
      ...browserWindowOptions,
      show: false,
    });
    this.mainWindow = window;
    this.windowStateTracking = windowsStateHandler.trackWindow(window);
    window.once("ready-to-show", () => {
      if (window.isDestroyed()) {
        return;
      }
      applyWindowState(window, windowState);
      window.show();
    });

    const rendererEntry = this.resolveRendererEntry("workbench");

    const windowDisposables = this.own(new DisposableStore());
    windowDisposables.add(this.windowStateTracking);
    const browserViewMainService = windowDisposables.add(
      new BrowserViewMainService({
        window,
        emitEvent: (event) => {
          if (!window.isDestroyed()) {
            window.webContents.send(BROWSER_VIEW_EVENT_CHANNEL, event);
          }
        },
      }),
    );
    windowDisposables.add(workspaceContext.onDidChangeWorkspace(({ workspace: nextWorkspace }) => {
      if (window.isDestroyed()) return;
      this.windowStateTracking?.dispose();
      const nextWindowsStateHandler = this.createWindowsStateHandler(nextWorkspace);
      this._windowsStateHandler = nextWindowsStateHandler;
      this.windowStateTracking = windowDisposables.add(nextWindowsStateHandler.trackWindow(window));
    }));
    windowDisposables.add(workspaceContext.onDidChangeWorkspace(({ workspace: nextWorkspace }) => {
      if (!window.isDestroyed()) {
        window.webContents.send(WORKSPACE_CONTEXT_CHANGED_CHANNEL, serializeWorkspaceIdentifier(nextWorkspace));
      }
    }));
    const { configuration, keybindings } = this.services;
    const workspaceTransitions = windowDisposables.add(new WorkspaceTransitionMainService({
      workspaces,
      context: workspaceContext,
      ...this.createWorkspaceTransitionRuntime(supervisor),
    }));
    const ipcRoutes = [
      ...appServerIpcRoutes(supervisor),
      ...sessionIpcRoutes(supervisor),
      ...typstIpcRoutes(supervisor),
      ...fileIpcRoutes(supervisor),
      ...extensionIpcRoutes(supervisor),
      ...diffIpcRoutes(supervisor),
      ...documentCollaborationIpcRoutes(supervisor),
      ...syntaxIpcRoutes(supervisor),
      ...gitIpcRoutes(supervisor),
      ...searchIpcRoutes(supervisor),
      ...terminalIpcRoutes(supervisor),
      ...browserViewIpcRoutes(browserViewMainService),
      ...configurationIpcRoutes(configuration),
      ...keybindingsResourceIpcRoutes(keybindings),
      ...nativeHostIpcRoutes({
        openFolder: async () => {
          const result = await dialog.showOpenDialog(window, {
            title: "Open Folder",
            properties: ["openDirectory"],
          });
          const folderPath = result.filePaths[0];
          if (result.canceled || !folderPath) return;
          await this.windowsStateHandler.saveWindowState(window);
          const transition = await workspaceTransitions.transitionToFolder(folderPath);
          if (transition.status === WorkspaceTransitionStatus.Blocked) {
            await dialog.showMessageBox(window, {
              type: "info",
              message: "Finish the active request before opening another folder.",
              detail: "The current Workspace was kept unchanged.",
            });
            return;
          }
          if (transition.status === WorkspaceTransitionStatus.Failed) {
            throw workspaceTransitionError(transition.failure);
          }
        },
        saveFile: async (options) => {
          const result = await dialog.showSaveDialog(window, {
            title: "Save File",
            ...(options.defaultName ? { defaultPath: options.defaultName } : {}),
          });
          return result.canceled || !result.filePath ? undefined : result.filePath;
        },
        isAccessibilitySupportEnabled: () => app.isAccessibilitySupportEnabled(),
        setWindowTheme: ({ backgroundColor, symbolColor }) => {
          if (process.platform === "win32" || process.platform === "linux") {
            window.setTitleBarOverlay({ color: backgroundColor, symbolColor, height: 35 });
          }
        },
        toggleDeveloperTools: () => window.webContents.toggleDevTools(),
      }),
      ...userThemeIpcRoutes(new UserThemeFileService(join(app.getPath("userData"), "themes"))),
      ...workspaceContextIpcRoutes(workspaceContext),
    ];
    if (this.product.dedicatedSessions) {
      ipcRoutes.push(...sessionsWindowIpcRoutes({
        openSessionsWindow: () => this.openSessionsWindow(supervisor),
        returnToWorkbench: () => this.focusMainWindow(),
      }));
    }
    if (process.platform === "darwin") {
      const nativeContextMenu = windowDisposables.add(
        new ElectronContextMenu(window),
      );
      const nativeMenubar = windowDisposables.add(
        new NativeMenubarMainService(window),
      );
      ipcRoutes.push(...nativeContextMenuIpcRoutes(nativeContextMenu));
      ipcRoutes.push(...nativeMenubarIpcRoutes(nativeMenubar));
    }
    windowDisposables.add(this.trustedIpcRouter.register(
      {
        webContents: window.webContents,
        allowedEntryUrls: new Set([
          normalizeEntryUrl(rendererEntry.url),
        ]),
      },
      ipcRoutes,
    ));
    windowDisposables.add(supervisor.onNotification((notification) =>
      window.webContents.send("zeta:event", notification)
    ));
    windowDisposables.add(supervisor.onStateChange((state) =>
      window.webContents.send("zeta:app-server:stateChanged", state)
    ));
    windowDisposables.add(configuration.onDidChange((snapshot) =>
      window.webContents.send(CONFIGURATION_CHANGED_CHANNEL, snapshot)
    ));
    windowDisposables.add(keybindings.onDidChange((snapshot) =>
      window.webContents.send(KEYBINDINGS_RESOURCE_CHANGED_CHANNEL, snapshot)
    ));
    window.once("closed", () => {
      windowDisposables.dispose();
      if (this.mainWindow === window) {
        this.closeSessionsWindow();
        this.mainWindow = undefined;
        this.windowStateTracking = undefined;
      }
    });

    if (rendererEntry.useDevelopmentUrl) {
      await window.loadURL(rendererEntry.url);
    } else {
      await window.loadFile(rendererEntry.file);
    }
  }

  /** Creates or focuses the one product-scoped Sessions window beside Workbench. */
  private async openSessionsWindow(supervisor: AppServerSupervisor): Promise<void> {
    if (!this.product.dedicatedSessions) {
      throw new Error(`${this.product.name} does not provide a dedicated Sessions window`);
    }
    const existing = this.sessionsWindow;
    if (existing && !existing.isDestroyed()) {
      if (existing.isMinimized()) existing.restore();
      existing.focus();
      return;
    }

    const sessionsEntry = this.resolveRendererEntry("sessions");
    const browserWindowOptions = resolveBrowserWindowOptions({
      state: {
        mode: WindowMode.Normal,
        width: 1_180,
        height: 780,
      },
      webPreferences: this.createSandboxWebPreferences(),
    });
    const window = new BrowserWindow({
      ...browserWindowOptions,
      show: false,
      title: `${this.product.name} Sessions`,
    });
    this.sessionsWindow = window;
    window.once("ready-to-show", () => {
      if (!window.isDestroyed()) {
        window.show();
      }
    });

    const windowDisposables = this.own(new DisposableStore());
    windowDisposables.add(this.trustedIpcRouter.register(
      {
        webContents: window.webContents,
        allowedEntryUrls: new Set([
          normalizeEntryUrl(sessionsEntry.url),
        ]),
      },
      [
        ...appServerIpcRoutes(supervisor),
        ...sessionIpcRoutes(supervisor),
        ...sessionsWindowIpcRoutes({
          openSessionsWindow: () => this.openSessionsWindow(supervisor),
          returnToWorkbench: () => this.returnToMainWindow(window),
        }),
      ],
    ));
    windowDisposables.add(supervisor.onNotification((notification) => {
      if (!window.isDestroyed()) {
        window.webContents.send("zeta:event", notification);
      }
    }));
    windowDisposables.add(supervisor.onStateChange((state) => {
      if (!window.isDestroyed()) {
        window.webContents.send("zeta:app-server:stateChanged", state);
      }
    }));
    window.once("closed", () => {
      windowDisposables.dispose();
      if (this.sessionsWindow === window) {
        this.sessionsWindow = undefined;
      }
    });

    try {
      if (sessionsEntry.useDevelopmentUrl) {
        await window.loadURL(sessionsEntry.url);
      } else {
        await window.loadFile(sessionsEntry.file);
      }
    } catch (error) {
      if (!window.isDestroyed()) {
        window.destroy();
      }
      throw error;
    }
  }

  private returnToMainWindow(sessionsWindow: BrowserWindow): void {
    this.focusMainWindow();
    if (!sessionsWindow.isDestroyed()) {
      sessionsWindow.close();
    }
  }

  private closeSessionsWindow(): void {
    const window = this.sessionsWindow;
    if (window && !window.isDestroyed()) {
      window.close();
    }
  }

  private resolveRendererEntry(kind: "workbench" | "sessions"): RendererEntry {
    const entry = kind === "workbench"
      ? this.product.rendererEntry
      : this.product.dedicatedSessions?.rendererEntry;
    if (!entry) {
      throw new Error(`${this.product.name} does not provide a Sessions renderer entry`);
    }
    const file = join(
      this.rendererRoot,
      this.product.id,
      "electron-browser",
      kind,
      `${entry}.html`,
    );
    const rendererUrl = process.env.ZETA_RENDERER_URL;
    const useDevelopmentUrl = !app.isPackaged && rendererUrl !== undefined;
    return {
      file,
      url: useDevelopmentUrl
        ? new URL(`/electron-browser/${kind}/${entry}.html`, rendererUrl).href
        : pathToFileURL(file).href,
      useDevelopmentUrl,
    };
  }

  private createSandboxWebPreferences() {
    return {
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
      preload: join(
        app.getAppPath(),
        "dist/preload/src/zeta/base/parts/sandbox/electron-browser/preload.cjs",
      ),
      additionalArguments: [],
    };
  }

  private createWorkspaceTransitionRuntime(
    supervisor: AppServerSupervisor,
  ): Pick<WorkspaceTransitionMainServiceOptions, "runtime" | "classifyRuntimeError" | "recovery"> {
    if (this.appServerStartupMode === "disabled") {
      return {
        runtime: {
          async switchWorkspace() {
            // UI-only development changes the window context without touching a backend runtime.
          },
        },
        classifyRuntimeError: () => WorkspaceTransitionFailureKind.RuntimeUnavailable,
      };
    }
    const appServerWorkspace = createAppServerWorkspaceTransitionAdapter(supervisor);
    return {
      runtime: appServerWorkspace,
      classifyRuntimeError: (error) => appServerWorkspace.classifyRuntimeError(error),
      recovery: appServerWorkspace,
    };
  }

  private readonly onBeforeQuit = (event: ElectronEvent): void => {
    this.quitRequested = true;
    this.supervisor?.dispose();
    if (this.quitAfterStateSaved || !this.persistentServices) {
      return;
    }
    event.preventDefault();
    if (this.quitSaveStarted) {
      return;
    }

    this.quitSaveStarted = true;
    this.windowStateTracking?.dispose();
    void (async () => {
      try {
        if (this.mainWindow && !this.mainWindow.isDestroyed()) {
          await this._windowsStateHandler?.saveWindowState(this.mainWindow);
        }
        await this.closePersistentServices();
      } catch (error) {
        console.error("Failed to flush application state before quit", error);
      } finally {
        this.quitAfterStateSaved = true;
        app.quit();
      }
    })();
  };

  private readonly onAccessibilitySupportChanged = (
    _event: ElectronEvent,
    enabled: boolean,
  ): void => {
    const window = this.mainWindow;
    if (!window || window.isDestroyed()) return;
    window.webContents.send(NATIVE_HOST_ACCESSIBILITY_SUPPORT_CHANGED_CHANNEL, enabled);
  };

  private readonly onWillQuit = (): void => {
    this.dispose();
    this.releaseDisposableTracker();
  };

  private closePersistentServices(): Promise<void> {
    const services = this.persistentServices;
    this.closePersistentServicesPromise ??= services
      ? Promise.all([
          services.state.close(),
          services.configuration.close(),
          services.keybindings.close(),
        ]).then(() => undefined)
      : Promise.resolve();
    return this.closePersistentServicesPromise;
  }

  private appServerEnvironment(
    workspace: IAnyWorkspaceIdentifier,
  ): Readonly<Record<string, string>> {
    return buildAppServerEnvironment(process.env, process.platform === "win32" ? "windows" : "posix", {
      ...(process.env.ZETA_RG_PATH
        ? { ZETA_RG_PATH: process.env.ZETA_RG_PATH }
        : {}),
      ZETA_PROFILE_ROOT: join(app.getPath("userData"), "state"),
      ...(isSingleFolderWorkspaceIdentifier(workspace)
        ? { ZETA_WORKSPACE_ROOT: workspace.uri.fsPath }
        : {}),
    });
  }

  private createWindowsStateHandler(
    workspace: IAnyWorkspaceIdentifier,
  ): WindowsStateHandler {
    return new WindowsStateHandler({
      stateService: this.services.state,
      workspace,
      displayService: {
        getAllDisplays: () => screen.getAllDisplays(),
        getDisplayMatching: (bounds) => screen.getDisplayMatching(bounds),
      },
      onError: (error) => {
        console.error("Failed to save window state", error);
      },
    });
  }

  private get services(): PersistentServices {
    assertDefined(this.persistentServices, "Persistent application services are not initialized");
    return this.persistentServices;
  }

  private get windowsStateHandler(): WindowsStateHandler {
    assertDefined(this._windowsStateHandler, "Window state handling is not initialized");
    return this._windowsStateHandler;
  }

  private releaseDisposableTracker(): void {
    try {
      this.disposableTracker?.assertNoLeaks();
    } finally {
      this.tracking?.[Symbol.dispose]();
    }
  }
}

function workspaceTransitionError(
  failure: IWorkspaceTransitionFailure | undefined,
): Error {
  if (!failure) return new Error("Workspace transition failed without a classified failure");
  if (failure.error instanceof Error) return failure.error;
  return new Error(`Workspace transition failed during ${failure.stage}`);
}
