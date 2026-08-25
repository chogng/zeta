import { shell } from "electron";
import { app, BrowserWindow, dialog, ipcMain, Menu, screen, type Event as ElectronEvent } from "electron/main";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { isCancellationError } from "../../base/common/cancellation.js";
import { DisposableOwner, DisposableStore, type IDisposable, toDisposable } from "../../base/common/lifecycle.js";
import { assertDefined } from "../../base/common/types.js";
import { DisposableTracker, installDisposableTracker } from "../../base/common/disposableTracker.js";
import { ZetaApplicationName } from '../common/application.js';
import { WorkbenchModeConfigurationKey, WorkbenchModeRegistry, WorkbenchRendererEntry, withWorkbenchModeId, type WorkbenchModeId } from "../../workbench/common/workbenchMode.js";
import { ElectronContextMenu } from "../../base/parts/contextmenu/electron-main/contextmenu.js";
import { appServerIpcRoutes } from "../../platform/app-server/electron-main/app-server-ipc.js";
import { buildAppServerEnvironment } from "../../platform/app-server/common/appServerEnvironment.js";
import { AppServerSupervisor } from "../../platform/app-server/electron-main/app-server-supervisor.js";
import { appServerDaemonExecutablePath, developmentServerHostGenerationPath, packagedServerHostSha256, serverHostExecutablePath } from "../../platform/server-host/electron-main/serverHostPackage.js";
import { DevelopmentServerHostReloader, readDevelopmentServerHostGenerationSync, selectDevelopmentServerHostExecutable } from "../../platform/server-host/electron-main/developmentServerHostReloader.js";
import { LocalAppServerProcessLauncher } from "../../platform/app-server/electron-main/localAppServerProcessLauncher.js";
import { normalizeEntryUrl, TrustedIpcRouter, type IpcRoute } from "../../platform/ipc/electron-main/trustedIpcRouter.js";
import { BROWSER_VIEW_EVENT_CHANNEL } from "../../platform/browser/common/browserView.js";
import { browserViewIpcRoutes } from "../../platform/browser/electron-main/browserViewIpc.js";
import { BrowserViewMainService } from "../../platform/browser/electron-main/browserViewMainService.js";
import { BrowserAutomationMainService, registerBrowserAutomationHost } from "../../platform/browser/electron-main/browserAutomationMainService.js";
import { BrowserTargetRegistry } from "../../platform/browser/electron-main/browserTargetRegistry.js";
import { CONFIGURATION_CHANGED_CHANNEL } from "../../platform/configuration/common/configurationIpc.js";
import { configurationValues } from '../../platform/configuration/common/configurationIpc.js';
import { editJsonObjectProperty } from '../../base/common/json.js';
import { ConfigurationMainService, configurationIpcRoutes } from "../../platform/configuration/electron-main/configurationMainService.js";
import { nativeContextMenuIpcRoutes } from "../../platform/contextview/electron-main/contextMenuIpc.js";
import { fileIpcRoutes } from "../../platform/files/electron-main/fileIpcRoutes.js";
import { extensionIpcRoutes } from "../../platform/extensions/electron-main/extensionIpcRoutes.js";
import { extensionHostIpcRoutes } from "../../platform/extensionHost/electron-main/extensionHostIpcRoutes.js";
import { developmentArtifactsPath } from "../../platform/environment/node/developmentArtifacts.js";
import { diffIpcRoutes } from "../../platform/diff/electron-main/diffIpcRoutes.js";
import { documentCollaborationIpcRoutes } from "../../platform/collaboration/electron-main/documentCollaborationIpcRoutes.js";
import { syntaxIpcRoutes } from "../../platform/syntax/electron-main/syntaxIpcRoutes.js";
import { languageIpcRoutes } from "../../platform/language/electron-main/languageIpcRoutes.js";
import { gitIpcRoutes } from "../../platform/git/electron-main/gitIpcRoutes.js";
import { codeIndexIpcRoutes } from "../../platform/codeIndex/electron-main/codeIndexIpcRoutes.js";
import { symbolIndexIpcRoutes } from "../../platform/symbolIndex/electron-main/symbolIndexIpcRoutes.js";
import { connectorIpcRoutes } from "../../platform/connectors/electron-main/connectorIpcRoutes.js";
import { pluginIpcRoutes } from "../../platform/plugins/electron-main/pluginIpcRoutes.js";
import { marketplaceIpcRoutes } from "../../platform/marketplace/electron-main/marketplaceIpcRoutes.js";
import { toolSearchIpcRoutes } from "../../platform/toolSearch/electron-main/toolSearchIpcRoutes.js";
import { workspaceTrustIpcRoutes } from "../../platform/workspaceTrust/electron-main/workspaceTrustIpcRoutes.js";
import { accountIpcRoutes } from "../../platform/accounts/electron-main/accountIpcRoutes.js";
import { ElectronClipboardService } from "../../platform/clipboard/electron-main/electronClipboardService.js";
import { ElectronOpenerService } from "../../platform/opener/electron-main/electronOpenerService.js";
import { KEYBINDINGS_RESOURCE_CHANGED_CHANNEL } from "../../platform/keybinding/common/keybindingsResource.js";
import { KeybindingsResourceMainService, keybindingsResourceIpcRoutes } from "../../platform/keybinding/electron-main/keybindingsResourceMainService.js";
import { NATIVE_KEYBOARD_LAYOUT_CHANGED_CHANNEL } from "../../platform/keyboardLayout/common/nativeKeyboardLayout.js";
import { NativeKeyboardLayoutMainService, nativeKeyboardLayoutIpcRoutes } from "../../platform/keyboardLayout/electron-main/nativeKeyboardLayoutMainService.js";
import { USER_KEYBOARD_LAYOUT_CHANGED_CHANNEL } from "../../platform/keyboardLayout/common/userKeyboardLayout.js";
import { UserKeyboardLayoutMainService, userKeyboardLayoutIpcRoutes } from "../../platform/keyboardLayout/electron-main/userKeyboardLayoutMainService.js";
import { NativeMenubarMainService, nativeMenubarIpcRoutes } from "../../platform/menubar/electron-main/menubarMainService.js";
import { nativeHostIpcRoutes } from "../../platform/native/electron-main/nativeHostIpc.js";
import { NATIVE_HOST_ACCESSIBILITY_SUPPORT_CHANGED_CHANNEL } from "../../platform/native/common/nativeHost.js";
import { searchIpcRoutes } from "../../platform/search/electron-main/searchIpcRoutes.js";
import { sessionIpcRoutes } from "../../platform/sessions/electron-main/sessionIpcRoutes.js";
import { skillIpcRoutes } from "../../platform/skills/electron-main/skillIpcRoutes.js";
import { sessionsWindowIpcRoutes } from "../../sessions/electron-main/sessionsWindowIpc.js";
import { StateService } from "../../platform/state/node/stateService.js";
import { localProfileRoot, migrateLegacyLocalProfile } from "../../platform/profile/node/localProfile.js";
import { terminalIpcRoutes } from "../../platform/terminal/electron-main/terminalIpcRoutes.js";
import { ReconnectableTerminalMainService } from "../../platform/terminal/electron-main/reconnectableTerminalMainService.js";
import { userThemeIpcRoutes } from "../../platform/theme/electron-main/userThemeIpc.js";
import { UserThemeFileService } from "../../platform/theme/node/userThemeFileService.js";
import { typstIpcRoutes } from "../../platform/typst/electron-main/typstIpcRoutes.js";
import { applyWindowState, resolveBrowserWindowOptions } from "../../platform/windows/electron-main/windows.js";
import { WindowMode } from "../../platform/window/electron-main/window.js";
import { WindowsStateHandler } from "../../platform/windows/electron-main/windowsStateHandler.js";
import { type IAnyWorkspaceIdentifier, isRemoteWorkspaceIdentifier, isSingleFolderWorkspaceIdentifier, serializeWorkspace, UNKNOWN_EMPTY_WINDOW_WORKSPACE } from "../../platform/workspace/common/workspace.js";
import { packagedRemoteRuntimeCatalogSource } from "../../platform/remote/electron-main/packagedRemoteRuntimeCatalog.js";
import { ServerHostRemoteRuntimeInstaller, remoteRuntimeArtifactFromEnvironment } from "../../platform/remote/electron-main/serverHostRemoteRuntimeInstaller.js";
import { ServerHostRemoteRuntimeProvisioner } from "../../platform/remote/electron-main/serverHostRemoteRuntimeProvisioner.js";
import { ServerHostRemoteConnectionProfiles } from "../../platform/remote/electron-main/serverHostRemoteConnectionProfiles.js";
import { ServerHostRemoteConnections } from "../../platform/remote/electron-main/serverHostRemoteConnections.js";
import type { RemoteConnectionDefinition } from "../../platform/remote/common/remoteConnectionService.js";
import { getRemoteAuthority, isRemoteResource } from "../../platform/remote/common/remote.js";
import { RemoteBrowserViewNavigationResolver } from "../../platform/remote/electron-main/remoteBrowserViewNavigationResolver.js";
import { SshRemoteTunnelService } from "../../platform/remote/electron-main/sshRemoteTunnelService.js";
import { createRemoteRuntimeInstallProgressLogger } from "../../platform/remote/electron-main/remoteRuntimeBootstrapMainService.js";
import { RemoteRuntimeBootstrapMainService } from "../../platform/remote/electron-main/remoteRuntimeBootstrapMainService.js";
import { SshAppServerProcessLauncher } from "../../platform/remote/electron-main/sshAppServerProcessLauncher.js";
import { ElectronRemoteRuntimeInstallWindow } from "../../platform/remote/electron-main/electronRemoteRuntimeInstallWindow.js";
import { electronRemoteWindowMainHost } from "../../platform/remote/electron-main/electronRemoteWindowMainHost.js";
import { RemoteWindowMainContext } from "../../platform/remote/electron-main/remoteWindowMainContext.js";
import { WORKSPACE_CONTEXT_CHANGED_CHANNEL } from "../../platform/workspace/common/workspaceIpc.js";
import { createAppServerWorkspaceTransitionAdapter, readAppServerWorkspaceTrust, setAppServerWorkspaceFolders, switchAppServerWorkspace } from "../../platform/workspaces/electron-main/appServerWorkspaceTransition.js";
import { type IWorkspaceTransitionFailure, type WorkspaceTransitionMainServiceOptions, WorkspaceTransitionFailureKind, WorkspaceTransitionMainService, WorkspaceTransitionStatus, WorkspaceTrustChoice } from "../../platform/workspaces/electron-main/workspaceTransitionMainService.js";
import { WorkspaceContextMainService, WorkspacesMainService, parseWorkspaceLaunchArguments, workspaceContextIpcRoutes } from "../../platform/workspaces/electron-main/workspacesMainService.js";
import type { IWorkbenchWindowRecord } from "./workbenchWindowRegistry.js";
import { WorkbenchWindowRegistry } from "./workbenchWindowRegistry.js";
import { electronWorkspaceLaunchArguments } from "./electronWindowLaunch.js";
import { workbenchModeIpcRoutes } from "../../workbench/services/workbenchMode/electron-main/workbenchModeIpc.js";
export type AppServerStartupMode = "required" | "disabled";
export type ElectronMainIpcRouteContribution = (supervisor: AppServerSupervisor) => readonly IpcRoute<unknown, unknown>[];

export interface ZetaApplicationOptions {
	readonly initialModeId: WorkbenchModeId;
	readonly rendererRoot: string;
	/** Selects whether this Electron process starts the App Server before opening its window. */
	readonly appServerStartupMode: AppServerStartupMode;
	/** Host-selected IPC capabilities installed for every Workbench window. */
	readonly ipcRouteContributions?: readonly ElectronMainIpcRouteContribution[];
}

interface PersistentServices {
	readonly state: StateService;
	readonly configuration: ConfigurationMainService;
	readonly keybindings: KeybindingsResourceMainService;
	readonly userKeyboardLayout: UserKeyboardLayoutMainService;
}

interface RendererEntry {
	readonly file: string;
	readonly url: string;
	readonly useDevelopmentUrl: boolean;
}

interface WorkbenchWindowRecord extends IWorkbenchWindowRecord {
	readonly window: BrowserWindow;
	readonly workspaceContext: WorkspaceContextMainService;
	readonly supervisor: AppServerSupervisor;
	readonly resources: DisposableStore;
	modeId: WorkbenchModeId;
	windowsStateHandler: WindowsStateHandler;
	windowStateTracking: IDisposable;
	sessionsWindow?: BrowserWindow;
	openWorkspace?: (root: string) => Promise<void>;
}

interface PendingWindowLaunch {
	readonly arguments: readonly string[];
	readonly cwd: string;
}

/**
 * Owns the Electron application's persistent services, Workbench windows, IPC, and shutdown.
 */
export class ZetaApplication extends DisposableOwner {
	private defaultModeId: WorkbenchModeId;
	private readonly rendererRoot: string;
	private readonly appServerStartupMode: AppServerStartupMode;
	private readonly ipcRouteContributions: readonly ElectronMainIpcRouteContribution[];
	private readonly disposableTracker: DisposableTracker | undefined;
	private readonly tracking: Disposable | undefined;
	private readonly trustedIpcRouter: TrustedIpcRouter;
	private readonly nativeKeyboardLayout: NativeKeyboardLayoutMainService;
	private readonly profileRoot: string;

	private readonly workbenchWindows = new WorkbenchWindowRegistry<WorkbenchWindowRecord>();
	private readonly pendingWindowLaunches: PendingWindowLaunch[] = [];
	private workspaces: WorkspacesMainService | undefined;
	private persistentServices: PersistentServices | undefined;
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
		this.defaultModeId = options.initialModeId;
		this.rendererRoot = options.rendererRoot;
		this.appServerStartupMode = options.appServerStartupMode;
		this.ipcRouteContributions = options.ipcRouteContributions ?? [];
		this.disposableTracker = disposableTracker;
		this.tracking = tracking;
		this.trustedIpcRouter = this.own(new TrustedIpcRouter(ipcMain));
		this.nativeKeyboardLayout = this.own(new NativeKeyboardLayoutMainService());
		this.profileRoot = localProfileRoot();

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
		this.workspaces = workspaces;
		const workspace = await this.resolveWorkspace(workspaces);
		const record = await this.openWorkspace(workspace, workspaces);
		if (!record) {
			if (!this.quitRequested) app.quit();
			return;
		}
		await this.drainPendingWindowLaunches();
	}

	async disposeAfterStartupFailure(): Promise<void> {
		for (const record of this.workbenchWindows.values()) record.resources.dispose();
		try {
			await this.closePersistentServices();
		} finally {
			this.dispose();
			this.releaseDisposableTracker();
		}
	}

	/** Brings the most recently active Workbench window to the foreground. */
	focusMainWindow(): void {
		this.workbenchWindows.focusActive();
	}

	/** Opens a second-instance Workspace in its own window, or focuses the active window when no target was supplied. */
	handleSecondInstance(arguments_: readonly string[], cwd: string): void {
		const launch = { arguments: arguments_, cwd };
		if (!this.persistentServices || !this.workspaces) {
			this.pendingWindowLaunches.push(launch);
			return;
		}
		void this.openWindowLaunch(launch).catch(error => this.reportWindowOpenFailure(error));
	}

	/** Recreates an empty Workbench after macOS activates an application with no open windows. */
	handleActivate(): void {
		if (this.workbenchWindows.focusActive()) return;
		const workspaces = this.workspaces;
		if (!workspaces || !this.persistentServices) return;
		void this.openWorkspace(UNKNOWN_EMPTY_WINDOW_WORKSPACE, workspaces).catch(error => this.reportWindowOpenFailure(error));
	}

	private async createPersistentServices(): Promise<void> {
		await migrateLegacyLocalProfile({ legacyUserDataRoot: app.getPath("userData"), profileRoot: this.profileRoot });
		const state = await StateService.create(
			join(app.getPath("userData"), "state.json"),
		);
		let configuration: ConfigurationMainService | undefined;
		let keybindings: KeybindingsResourceMainService | undefined;
		let userKeyboardLayout: UserKeyboardLayoutMainService | undefined;
		try {
			configuration = await ConfigurationMainService.create({
				filePath: join(this.profileRoot, "configuration.json"),
				onError: (error) => {
					console.error("Failed to process configuration", error);
				},
			});
			keybindings = await KeybindingsResourceMainService.create({
				filePath: join(this.profileRoot, "keybindings.json"),
				onError: (error) => {
					console.error("Failed to process keybindings resource", error);
				},
			});
			userKeyboardLayout = await UserKeyboardLayoutMainService.create({
				filePath: join(this.profileRoot, "keyboard-layout.json"),
				openResource: (filePath) => shell.openPath(filePath),
				onError: (error) => {
					console.error("Failed to process user keyboard layout", error);
				},
			});
			this.persistentServices = { state, configuration, keybindings, userKeyboardLayout };
		} catch (error) {
			await Promise.all([
				state.close(),
				configuration?.close(),
				keybindings?.close(),
				userKeyboardLayout?.close(),
			]);
			throw error;
		}
	}

	private async resolveWorkspace(
		workspaces: WorkspacesMainService,
	): Promise<IAnyWorkspaceIdentifier> {
		try {
			return await workspaces.resolveStartupWorkspace({
				arguments: this.workspaceLaunchArguments(process.argv),
				cwd: process.cwd(),
			});
		} catch (error) {
			console.error("Failed to resolve startup workspace", error);
			return UNKNOWN_EMPTY_WINDOW_WORKSPACE;
		}
	}

	private workspaceLaunchArguments(arguments_: readonly string[]): string[] {
		return electronWorkspaceLaunchArguments({
			arguments: arguments_,
			packaging: app.isPackaged ? "packaged" : "development",
			appPath: app.getAppPath(),
		});
	}

	private async openWindowLaunch(launch: PendingWindowLaunch): Promise<void> {
		const workspaces = this.workspaces;
		if (!workspaces) throw new Error("Workspace service is not initialized");
		const arguments_ = this.workspaceLaunchArguments(launch.arguments);
		if (!parseWorkspaceLaunchArguments(arguments_)) {
			this.focusMainWindow();
			return;
		}
		const workspace = await workspaces.resolveStartupWorkspace({ arguments: arguments_, cwd: launch.cwd });
		await this.openWorkspace(workspace, workspaces);
	}

	private async drainPendingWindowLaunches(): Promise<void> {
		while (this.pendingWindowLaunches.length > 0 && !this.quitRequested) {
			const launch = this.pendingWindowLaunches.shift()!;
			try {
				await this.openWindowLaunch(launch);
			} catch (error) {
				await this.reportWindowOpenFailure(error);
			}
		}
	}

	private openWorkspace(workspace: IAnyWorkspaceIdentifier, workspaces: WorkspacesMainService): Promise<WorkbenchWindowRecord | undefined> {
		return this.workbenchWindows.openWorkspace(workspace.id, () => this.performOpenWorkspace(workspace, workspaces));
	}

	private async performOpenWorkspace(workspace: IAnyWorkspaceIdentifier, workspaces: WorkspacesMainService): Promise<WorkbenchWindowRecord | undefined> {
		const resources = this.own(new DisposableStore());
		try {
			const resolvedWorkspace = await workspaces.resolveWorkspace(workspace);
			const workspaceContext = resources.add(new WorkspaceContextMainService(workspace, resolvedWorkspace));
			const supervisor = resources.add(this.createAppServerSupervisor(workspace, resources));
			const browserAutomation = new BrowserAutomationMainService();
			resources.add(registerBrowserAutomationHost(supervisor, browserAutomation));
			resources.add(supervisor.onStateChange(state => {
				if (state === "crashed" || state === "restarting" || state === "stopping" || state === "stopped") browserAutomation.reset();
			}));
			if (this.appServerStartupMode === "required" && !await this.startAppServerWithRecovery(supervisor)) {
				resources.dispose();
				return undefined;
			}
			if (this.appServerStartupMode === "required" && resolvedWorkspace.folders.length > 0 && resolvedWorkspace.folders.every(folder => !isRemoteResource(folder.uri))) {
				const folders = [];
				for (const folder of resolvedWorkspace.folders) {
					const trust = await this.resolveLocalWorkspaceTrust(supervisor, folder.uri.fsPath);
					if (trust === undefined) {
						resources.dispose();
						return undefined;
					}
					folders.push({ id: folder.id, root: folder.uri.fsPath, trust });
				}
				await setAppServerWorkspaceFolders(supervisor, folders);
			}
			const existing = this.workbenchWindows.findWorkspace(workspace.id);
			if (existing) {
				existing.focus();
				resources.dispose();
				return existing;
			}
			return await this.openWorkbenchWindow(workspaceContext, workspaces, supervisor, browserAutomation, resources);
		} catch (error) {
			resources.dispose();
			throw error;
		}
	}

	private createAppServerSupervisor(
		workspace: IAnyWorkspaceIdentifier,
		resources: DisposableStore,
	): AppServerSupervisor {
		const packageLocation = {
			appPath: app.getAppPath(),
			expectedVersion: app.getVersion(),
			isPackaged: app.isPackaged,
			platform: process.platform,
			resourcesPath: process.resourcesPath,
		};
		const packagedExecutable = serverHostExecutablePath(packageLocation);
		const expectedPackagedSha256 = packagedServerHostSha256(packageLocation);
		const generationFile = !app.isPackaged && process.env.ZETA_DEV_SERVER_HOST_RELOAD === "1"
			? developmentServerHostGenerationPath(app.getAppPath())
			: undefined;
		let developmentExecutable: string | undefined;
		if (generationFile) {
			try {
				developmentExecutable = readDevelopmentServerHostGenerationSync(generationFile);
			} catch (error) {
				console.error("[server-host] Ignoring invalid development generation", error);
			}
		}
		const processLauncher = isRemoteWorkspaceIdentifier(workspace)
			? this.createSshAppServerProcessLauncher(workspace, resources)
			: new LocalAppServerProcessLauncher({
				executable: selectDevelopmentServerHostExecutable(packagedExecutable, developmentExecutable),
				expectedSha256: expectedPackagedSha256,
				args: ["app-server", "connect"],
				environment: this.appServerEnvironment(workspace),
			});
		const supervisor = new AppServerSupervisor({
			processLauncher,
			session: {
				clientName: "zeta-desktop",
				clientVersion: app.getVersion(),
				initializeTimeoutMs: 10_000,
				expectedServerName: "zeta-app-server",
				capabilities: {
					browser: { version: 1, observe: true, input: true },
					workspaceTrustHost: { version: 1 },
				},
			},
		});
		if (generationFile && processLauncher instanceof LocalAppServerProcessLauncher) {
			resources.add(new DevelopmentServerHostReloader({ generationFile, launcher: processLauncher, supervisor }));
		}
		return supervisor;
	}

	private createSshAppServerProcessLauncher(workspace: IAnyWorkspaceIdentifier, resources: DisposableStore) {
		if (!isRemoteWorkspaceIdentifier(workspace)) throw new Error("SSH App Server launcher requires a Remote workspace");
		const sshExecutable = process.env.ZETA_SSH_PATH ?? "ssh";
		const serverHostExecutable = serverHostExecutablePath({
			appPath: app.getAppPath(),
			isPackaged: app.isPackaged,
			platform: process.platform,
			resourcesPath: process.resourcesPath,
		});
		const artifact = remoteRuntimeArtifactFromEnvironment(process.env);
		const runtimeInstaller = artifact === undefined
			? new ServerHostRemoteRuntimeProvisioner({
				source: packagedRemoteRuntimeCatalogSource(
					{ appPath: app.getAppPath(), isPackaged: app.isPackaged, resourcesPath: process.resourcesPath },
					join(app.getPath("userData"), "remote-runtime-downloads"),
				),
				serverHostExecutable,
				sshExecutable,
				environment: process.env,
				installRoot: process.env.ZETA_REMOTE_RUNTIME_INSTALL_ROOT,
			})
			: new ServerHostRemoteRuntimeInstaller({
				serverHostExecutable,
				sshExecutable,
				environment: process.env,
				artifact,
				installRoot: process.env.ZETA_REMOTE_RUNTIME_INSTALL_ROOT,
			});
		const configuredRuntime = process.env.ZETA_REMOTE_ZETA_PATH;
		const connectionProfiles = configuredRuntime === undefined ? new ServerHostRemoteConnectionProfiles({
			serverHostExecutable,
			environment: { ...process.env, ZETA_PROFILE_ROOT: this.profileRoot },
		}) : undefined;
		const bootstrap = resources.add(new RemoteRuntimeBootstrapMainService({
			workspace: workspace.uri,
			sshExecutable,
			remoteExecutable: configuredRuntime ?? "zeta",
			localEnvironment: process.env,
			runtimeInstaller,
			connectionProfiles,
			logProgress: createRemoteRuntimeInstallProgressLogger(),
		}));
		resources.add(new ElectronRemoteRuntimeInstallWindow({
			productName: WorkbenchModeRegistry.get(this.defaultModeId).title,
			rendererEntry: this.resolveRendererEntry("remoteRuntimeInstall"),
			webPreferences: this.createSandboxWebPreferences(),
			trustedIpcRouter: this.trustedIpcRouter,
			progress: bootstrap.installProgress,
		}));
		return bootstrap.processLauncher;
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
				if (isCancellationError(error)) {
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
					title: `${WorkbenchModeRegistry.get(this.defaultModeId).title} startup failed`,
					message: "The App Server could not be validated.",
					detail,
					buttons: ["Retry", this.workbenchWindows.size === 0 ? "Quit" : "Cancel"],
					defaultId: 0,
					cancelId: 1,
					noLink: true,
				});
				if (this.quitRequested || result.response !== 0) {
					return false;
				}
				await supervisor.stop();
			}
		}
		return false;
	}

	private async openWorkbenchWindow(
		workspaceContext: WorkspaceContextMainService,
		workspaces: WorkspacesMainService,
		supervisor: AppServerSupervisor,
		browserAutomationMainService: BrowserAutomationMainService,
		resources: DisposableStore,
	): Promise<WorkbenchWindowRecord> {
		const windowsStateHandler = this.createWindowsStateHandler(workspaceContext.getWorkspace());
		const windowState = windowsStateHandler.restoreWindowState();
		const browserWindowOptions = resolveBrowserWindowOptions({
			state: windowState,
			webPreferences: this.createSandboxWebPreferences(),
		});
		const window = new BrowserWindow({
			...browserWindowOptions,
			show: false,
		});
		const windowStateTracking = windowsStateHandler.trackWindow(window);
		const record: WorkbenchWindowRecord = {
			id: window.id,
			workspaceId: workspaceContext.getWorkspace().id,
			window,
			workspaceContext,
			supervisor,
			resources,
			modeId: this.defaultModeId,
			windowsStateHandler,
			windowStateTracking,
			isDestroyed: () => window.isDestroyed(),
			focus: () => focusElectronWindow(window),
		};
		this.workbenchWindows.add(record);
		resources.defer(() => {
			if (!window.isDestroyed()) window.destroy();
		});
		const onFocus = (): void => {
			if (!window.isDestroyed()) this.workbenchWindows.activate(record.id);
		};
		window.on("focus", onFocus);
		resources.add(toDisposable(() => window.removeListener("focus", onFocus)));
		window.once("closed", () => {
			this.closeSessionsWindow(record);
			this.workbenchWindows.remove(record.id);
			if (!resources.disposed) resources.dispose();
		});
		window.once("ready-to-show", () => {
			if (window.isDestroyed()) {
				return;
			}
			applyWindowState(window, windowState);
			window.show();
		});

		const rendererEntry = this.resolveRendererEntry("workbench");

		const windowDisposables = resources;
		windowDisposables.add(record.windowStateTracking);
		const remoteTunnelService = new SshRemoteTunnelService({
			getWorkspace: () => workspaceContext.getWorkspace(),
			sshExecutable: process.env.ZETA_SSH_PATH ?? "ssh",
			localEnvironment: process.env,
		});
		const browserTargetRegistry = new BrowserTargetRegistry();
		const browserViewMainService = windowDisposables.add(
			new BrowserViewMainService({
				window,
				registry: browserTargetRegistry,
				emitEvent: (event) => {
					if (!window.isDestroyed()) {
						window.webContents.send(BROWSER_VIEW_EVENT_CHANNEL, event);
					}
				},
				navigationResolver: new RemoteBrowserViewNavigationResolver({
					getWorkspace: () => workspaceContext.getWorkspace(),
					tunnels: remoteTunnelService,
					reportError: (message, error) => console.error(message, error),
				}),
			}),
		);
		windowDisposables.add(browserAutomationMainService.bind(browserViewMainService, browserTargetRegistry));
		windowDisposables.add(workspaceContext.onDidChangeWorkspace(({ workspace: nextWorkspace }) => {
			if (window.isDestroyed()) return;
			record.windowStateTracking.dispose();
			const nextWindowsStateHandler = this.createWindowsStateHandler(nextWorkspace);
			record.windowsStateHandler = nextWindowsStateHandler;
			record.windowStateTracking = windowDisposables.add(nextWindowsStateHandler.trackWindow(window));
			this.workbenchWindows.updateWorkspace(record.id, nextWorkspace.id);
		}));
		windowDisposables.add(workspaceContext.onDidChangeWorkspace(({ resolvedWorkspace }) => {
			if (!window.isDestroyed()) {
				window.webContents.send(WORKSPACE_CONTEXT_CHANGED_CHANNEL, serializeWorkspace(resolvedWorkspace));
			}
		}));
		const { configuration, keybindings } = this.services;
		const workspaceTransitions = windowDisposables.add(new WorkspaceTransitionMainService({
			workspaces,
			context: workspaceContext,
			...this.createWorkspaceTransitionRuntime(supervisor),
		}));
		const remoteConnections = new ServerHostRemoteConnections({
			serverHostExecutable: serverHostExecutablePath({ appPath: app.getAppPath(), isPackaged: app.isPackaged, platform: process.platform, resourcesPath: process.resourcesPath }),
			environment: { ...process.env, ZETA_PROFILE_ROOT: this.profileRoot },
			scheduleConnect: connection => this.openRemoteConnection(connection, workspaces),
		});
		const reconnectableTerminals = isRemoteWorkspaceIdentifier(workspaceContext.getWorkspace())
			? windowDisposables.add(new ReconnectableTerminalMainService({ supervisor }))
			: undefined;
		const remoteWindowContext = windowDisposables.add(new RemoteWindowMainContext({
			supervisor,
			workspaceContext,
			connections: remoteConnections,
			tunnels: remoteTunnelService,
			host: electronRemoteWindowMainHost(window),
			...(reconnectableTerminals ? { prepareForRuntimeReplacement: () => reconnectableTerminals.prepareForServerReplacement() } : {}),
		}));
		const transitionToFolder = async (folderPath: string, trustRequired: boolean): Promise<void> => {
			const currentWorkspace = workspaceContext.getWorkspace();
			const nextWorkspace = isRemoteWorkspaceIdentifier(currentWorkspace)
				? await this.resolveRemoteFolderWorkspace(currentWorkspace, folderPath, workspaces)
				: await workspaces.resolveFolder(folderPath);
			if (nextWorkspace.id === currentWorkspace.id) return;
			const trust = await this.resolveLocalWorkspaceTrust(supervisor, folderPath, window);
			if (trust === undefined) {
				if (trustRequired) throw new Error(`Workspace trust was not selected for '${folderPath}'`);
				return;
			}
			await record.windowsStateHandler.saveWindowState(window);
			reconnectableTerminals?.prepareForServerReplacement();
			const transition = isRemoteWorkspaceIdentifier(nextWorkspace)
				? await workspaceTransitions.transitionToWorkspace({ workspace: nextWorkspace, root: folderPath }, trust)
				: await workspaceTransitions.transitionToFolder(folderPath, trust);
			if (transition.status === WorkspaceTransitionStatus.Blocked) {
				throw new Error("Finish the active request before opening another Workspace");
			}
			if (transition.status === WorkspaceTransitionStatus.Failed) {
				throw workspaceTransitionError(transition.failure);
			}
		};
		record.openWorkspace = (root) => transitionToFolder(root, true);
		const ipcRoutes = [
			...appServerIpcRoutes(supervisor),
			...accountIpcRoutes(supervisor, {
				openerService: new ElectronOpenerService(),
				clipboardService: new ElectronClipboardService(),
			}),
			...remoteWindowContext.ipcRoutes,
			...sessionIpcRoutes(supervisor),
			...skillIpcRoutes(supervisor),
			...typstIpcRoutes(supervisor),
			...fileIpcRoutes(supervisor),
			...extensionIpcRoutes(supervisor),
			...extensionHostIpcRoutes(supervisor),
			...diffIpcRoutes(supervisor),
			...documentCollaborationIpcRoutes(supervisor),
			...syntaxIpcRoutes(supervisor),
			...languageIpcRoutes(supervisor),
			...gitIpcRoutes(supervisor),
			...codeIndexIpcRoutes(supervisor),
			...symbolIndexIpcRoutes(supervisor),
			...connectorIpcRoutes(supervisor),
			...pluginIpcRoutes(supervisor),
			...marketplaceIpcRoutes(supervisor),
			...toolSearchIpcRoutes(supervisor),
			...workspaceTrustIpcRoutes(supervisor),
			...searchIpcRoutes(supervisor),
			...terminalIpcRoutes(supervisor, reconnectableTerminals),
			...this.ipcRouteContributions.flatMap(contribution => contribution(supervisor)),
			...browserViewIpcRoutes(browserViewMainService),
			...configurationIpcRoutes(configuration),
			...keybindingsResourceIpcRoutes(keybindings),
			...nativeKeyboardLayoutIpcRoutes(this.nativeKeyboardLayout),
			...userKeyboardLayoutIpcRoutes(this.services.userKeyboardLayout),
			...nativeHostIpcRoutes({
				openFolder: async () => {
					const result = await dialog.showOpenDialog(window, {
						title: "Open Folder",
						properties: ["openDirectory"],
					});
					const folderPath = result.filePaths[0];
					if (result.canceled || !folderPath) return;
					try {
						await transitionToFolder(folderPath, false);
					} catch (error) {
						if (error instanceof Error && error.message.includes("Finish the active request")) {
							await dialog.showMessageBox(window, {
								type: "info",
								message: "Finish the active request before opening another folder.",
								detail: "The current Workspace was kept unchanged.",
							});
							return;
						}
						throw error;
					}
				},
				pickFolder: async () => {
					const result = await dialog.showOpenDialog(window, {
						title: "Add Trusted Folder",
						properties: ["openDirectory"],
					});
					return result.canceled || !result.filePaths[0] ? undefined : result.filePaths[0];
				},
				openWorkspace: (root) => transitionToFolder(root, true),
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
			...workbenchModeIpcRoutes(modeId => this.scheduleWorkbenchModeSwitch(record, modeId)),
			...userThemeIpcRoutes(new UserThemeFileService(join(this.profileRoot, "themes"))),
			...workspaceContextIpcRoutes(workspaceContext),
		];
		ipcRoutes.push(...sessionsWindowIpcRoutes({
			openSessionsWindow: () => this.openSessionsWindow(record),
			returnToWorkbench: () => record.focus(),
			openWorkspace: (root) => record.openWorkspace?.(root) ?? Promise.reject(new Error("Workspace routing is unavailable")),
		}));
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
				allowedEntryUrls: new Set(WorkbenchModeRegistry.modeIds.map(modeId => normalizeEntryUrl(this.resolveRendererEntry("workbench", modeId).url))),
			},
			ipcRoutes,
		));
		windowDisposables.add(supervisor.onNotification((notification) =>
			window.webContents.send("zeta:event", notification)
		));
		windowDisposables.add(supervisor.onStateChange((state) => {
			window.webContents.send("zeta:app-server:stateChanged", state);
		}));
		windowDisposables.add(configuration.onDidChange((snapshot) =>
			window.webContents.send(CONFIGURATION_CHANGED_CHANNEL, snapshot)
		));
		windowDisposables.add(keybindings.onDidChange((snapshot) =>
			window.webContents.send(KEYBINDINGS_RESOURCE_CHANGED_CHANNEL, snapshot)
		));
		windowDisposables.add(this.nativeKeyboardLayout.onDidChangeKeyboardLayout((layout) =>
			window.webContents.send(NATIVE_KEYBOARD_LAYOUT_CHANGED_CHANNEL, layout)
		));
		windowDisposables.add(this.services.userKeyboardLayout.onDidChangeKeyboardLayout(() =>
			window.webContents.send(USER_KEYBOARD_LAYOUT_CHANGED_CHANNEL, this.services.userKeyboardLayout.currentKeyboardLayout)
		));
		try {
			await this.loadRendererEntry(window, rendererEntry);
			return record;
		} catch (error) {
			if (!window.isDestroyed()) window.destroy();
			throw error;
		}
	}

	/** Creates or focuses the Sessions window belonging to one Workbench supervisor. */
	private async openSessionsWindow(record: WorkbenchWindowRecord): Promise<void> {
		const mode = WorkbenchModeRegistry.get(record.modeId);
		if (!mode.dedicatedSessions) {
			throw new Error(`${mode.title} does not provide a dedicated Sessions window`);
		}
		const existing = record.sessionsWindow;
		if (existing && !existing.isDestroyed()) {
			if (existing.isMinimized()) existing.restore();
			existing.focus();
			return;
		}

		const sessionsEntry = this.resolveRendererEntry("sessions", record.modeId);
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
			title: `${mode.title} Sessions`,
		});
		record.sessionsWindow = window;
		window.once("ready-to-show", () => {
			if (!window.isDestroyed()) {
				window.show();
			}
		});

		const windowDisposables = record.resources.add(new DisposableStore());
		windowDisposables.add(this.trustedIpcRouter.register(
			{
				webContents: window.webContents,
				allowedEntryUrls: new Set([
					normalizeEntryUrl(sessionsEntry.url),
				]),
			},
			[
				...appServerIpcRoutes(record.supervisor),
				...sessionIpcRoutes(record.supervisor),
				...skillIpcRoutes(record.supervisor),
				...workspaceContextIpcRoutes(record.workspaceContext),
				...sessionsWindowIpcRoutes({
					openSessionsWindow: () => this.openSessionsWindow(record),
					returnToWorkbench: () => this.returnToMainWindow(record, window),
					openWorkspace: (root) => record.openWorkspace?.(root) ?? Promise.reject(new Error("Workspace routing is unavailable")),
				}),
			],
		));
		windowDisposables.add(record.supervisor.onNotification((notification) => {
			if (!window.isDestroyed()) {
				window.webContents.send("zeta:event", notification);
			}
		}));
		windowDisposables.add(record.workspaceContext.onDidChangeWorkspace(({ resolvedWorkspace }) => {
			if (!window.isDestroyed()) {
				window.webContents.send(WORKSPACE_CONTEXT_CHANGED_CHANNEL, serializeWorkspace(resolvedWorkspace));
			}
		}));
		windowDisposables.add(record.supervisor.onStateChange((state) => {
			if (!window.isDestroyed()) {
				window.webContents.send("zeta:app-server:stateChanged", state);
			}
		}));
		window.once("closed", () => {
			windowDisposables.dispose();
			if (record.sessionsWindow === window) record.sessionsWindow = undefined;
		});

		try {
			await this.loadRendererEntry(window, sessionsEntry);
		} catch (error) {
			if (!window.isDestroyed()) {
				window.destroy();
			}
			throw error;
		}
	}

	private returnToMainWindow(record: WorkbenchWindowRecord, sessionsWindow: BrowserWindow): void {
		record.focus();
		if (!sessionsWindow.isDestroyed()) {
			sessionsWindow.close();
		}
	}

	private closeSessionsWindow(record: WorkbenchWindowRecord): void {
		const window = record.sessionsWindow;
		if (window && !window.isDestroyed()) {
			window.close();
		}
	}

	private scheduleWorkbenchModeSwitch(record: WorkbenchWindowRecord, modeId: WorkbenchModeId): void {
		if (record.modeId === modeId) return;
		setImmediate(() => {
			void this.switchWorkbenchMode(record, modeId).catch(error => this.reportWorkbenchModeSwitchFailure(record, error));
		});
	}

	private async switchWorkbenchMode(record: WorkbenchWindowRecord, modeId: WorkbenchModeId): Promise<void> {
		if (record.window.isDestroyed() || record.modeId === modeId) return;
		const previousModeId = record.modeId;
		const previousDefaultModeId = this.defaultModeId;
		this.closeSessionsWindow(record);
		record.modeId = modeId;
		this.defaultModeId = modeId;
		try {
			await this.loadRendererEntry(record.window, this.resolveRendererEntry("workbench", modeId));
		} catch (error) {
			record.modeId = previousModeId;
			this.defaultModeId = previousDefaultModeId;
			let persistenceError: unknown;
			try {
				await this.persistWorkbenchModeId(previousDefaultModeId);
			} catch (candidate) {
				persistenceError = candidate;
			}
			try {
				await this.loadRendererEntry(record.window, this.resolveRendererEntry("workbench", previousModeId));
			} catch (rollbackError) {
				throw new AggregateError([error, persistenceError, rollbackError].filter(candidate => candidate !== undefined), "Workbench mode switch and rollback both failed");
			}
			if (persistenceError !== undefined) throw new AggregateError([error, persistenceError], "Workbench mode switch failed and its persisted preference could not be restored");
			throw error;
		}
	}

	private async persistWorkbenchModeId(modeId: WorkbenchModeId): Promise<void> {
		const configuration = this.services.configuration;
		const snapshot = configuration.read();
		if (configurationValues(snapshot.document)[WorkbenchModeConfigurationKey] === modeId) return;
		await configuration.update({
			expectedRevision: snapshot.revision,
			document: {
				version: 1,
				source: editJsonObjectProperty(snapshot.document.source, WorkbenchModeConfigurationKey, modeId),
			},
		});
	}

	private async reportWorkbenchModeSwitchFailure(record: WorkbenchWindowRecord, error: unknown): Promise<void> {
		console.error("Failed to switch Workbench mode", error);
		if (record.window.isDestroyed()) return;
		const detail = error instanceof Error ? error.message : "The requested mode could not be loaded";
		await dialog.showMessageBox(record.window, {
			type: "error",
			title: `${ZetaApplicationName} mode switch failed`,
			message: "The requested Workbench mode could not be loaded.",
			detail: detail.slice(0, 8_000),
			buttons: ["OK"],
			defaultId: 0,
			cancelId: 0,
			noLink: true,
		});
	}

	private resolveRendererEntry(kind: "workbench" | "sessions" | "remoteRuntimeInstall", modeId: WorkbenchModeId = this.defaultModeId): RendererEntry {
		const mode = WorkbenchModeRegistry.get(modeId);
		const entry = kind === "workbench"
			? WorkbenchRendererEntry
			: kind === "sessions"
				? mode.dedicatedSessions?.rendererEntry
				: "remoteRuntimeInstall";
		if (!entry) {
			throw new Error(`${mode.title} does not provide a Sessions renderer entry`);
		}
		const directory = kind === "remoteRuntimeInstall" ? "remote-runtime-install" : kind;
		const file = join(
			this.rendererRoot,
			"electron-browser",
			directory,
			`${entry}.html`,
		);
		const rendererUrl = process.env.ZETA_RENDERER_URL;
		const useDevelopmentUrl = !app.isPackaged && rendererUrl !== undefined;
		const baseUrl = useDevelopmentUrl
			? new URL(`/electron-browser/${directory}/${entry}.html`, rendererUrl).href
			: pathToFileURL(file).href;
		return {
			file,
			url: kind === "workbench" ? withWorkbenchModeId(baseUrl, modeId) : baseUrl,
			useDevelopmentUrl,
		};
	}

	private async loadRendererEntry(window: BrowserWindow, entry: RendererEntry): Promise<void> {
		if (entry.useDevelopmentUrl || new URL(entry.url).search.length > 0) {
			await window.loadURL(entry.url);
			return;
		}
		await window.loadFile(entry.file);
	}

	private createSandboxWebPreferences() {
		return {
			contextIsolation: true,
			nodeIntegration: false,
			sandbox: true,
			preload: app.isPackaged
				? join(app.getAppPath(), "dist/preload/src/zeta/base/parts/sandbox/electron-browser/preload.cjs")
				: developmentArtifactsPath(app.getAppPath(), "preload", "src/zeta/base/parts/sandbox/electron-browser/preload.cjs"),
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
		const launcher = supervisor.options.processLauncher;
		const appServerWorkspace = createAppServerWorkspaceTransitionAdapter(
			supervisor,
			launcher instanceof LocalAppServerProcessLauncher
				? (root, trust) => this.reconnectLocalAppServerWorkspace(supervisor, launcher, root, trust)
				: launcher instanceof SshAppServerProcessLauncher
					? (root, trust) => this.reconnectRemoteAppServerWorkspace(supervisor, launcher, root, trust)
				: undefined,
		);
		return {
			runtime: appServerWorkspace,
			classifyRuntimeError: (error) => appServerWorkspace.classifyRuntimeError(error),
			recovery: appServerWorkspace,
		};
	}

	private async reconnectLocalAppServerWorkspace(
		supervisor: AppServerSupervisor,
		launcher: LocalAppServerProcessLauncher,
		root: string,
		trust: WorkspaceTrustChoice,
	): Promise<void> {
		const previousEnvironment = launcher.environment;
		const nextEnvironment = {
			...previousEnvironment,
			ZETA_WORKSPACE_ROOT: root,
			ZETA_WORKSPACE_TRUST_SOURCE: "userConfig",
		};
		await supervisor.stop();
		launcher.replaceEnvironment(nextEnvironment);
		try {
			await supervisor.start();
			await switchAppServerWorkspace(supervisor, root, trust);
		} catch (error) {
			await supervisor.stop();
			launcher.replaceEnvironment(previousEnvironment);
			try {
				await supervisor.start();
			} catch (rollbackError) {
				throw new AggregateError([error, rollbackError], "Workspace authority switch and rollback both failed");
			}
			throw error;
		}
	}

	private async reconnectRemoteAppServerWorkspace(
		supervisor: AppServerSupervisor,
		launcher: SshAppServerProcessLauncher,
		root: string,
		trust: WorkspaceTrustChoice,
	): Promise<void> {
		const previousRoot = launcher.workspaceRoot;
		await supervisor.stop();
		launcher.replaceWorkspaceRoot(root);
		try {
			await supervisor.start();
			await switchAppServerWorkspace(supervisor, root, trust);
		} catch (error) {
			await supervisor.stop();
			launcher.replaceWorkspaceRoot(previousRoot);
			try {
				await supervisor.start();
			} catch (rollbackError) {
				throw new AggregateError([error, rollbackError], "Remote Workspace authority switch and rollback both failed");
			}
			throw error;
		}
	}

	private async resolveRemoteFolderWorkspace(
		currentWorkspace: IAnyWorkspaceIdentifier,
		folderPath: string,
		workspaces: WorkspacesMainService,
	) {
		if (!isRemoteWorkspaceIdentifier(currentWorkspace)) throw new Error("Remote Workspace resolution requires a Remote window");
		const authority = getRemoteAuthority(currentWorkspace.uri);
		if (!authority || authority.type !== "ssh") throw new Error("Unsupported Remote Workspace authority");
		const workspace = await workspaces.resolveStartupWorkspace({
			arguments: ["--remote-ssh", authority.host, "--folder", folderPath],
			cwd: process.cwd(),
		});
		if (!isSingleFolderWorkspaceIdentifier(workspace) || !isRemoteWorkspaceIdentifier(workspace)) {
			throw new Error("Remote folder did not resolve to a Remote Workspace");
		}
		return workspace;
	}

	private async openRemoteConnection(connection: RemoteConnectionDefinition, workspaces: WorkspacesMainService): Promise<void> {
		if (this.quitRequested) return;
		const workspace = await workspaces.resolveStartupWorkspace({
			arguments: ["--remote-ssh", connection.host, "--folder", connection.workspace],
			cwd: process.cwd(),
		});
		await this.openWorkspace(workspace, workspaces);
	}

	private async reportWindowOpenFailure(error: unknown): Promise<void> {
		console.error("Failed to open Workbench window", error);
		if (this.quitRequested) return;
		const message = error instanceof Error ? error.message : "The requested Workspace could not be opened";
		try {
			await dialog.showMessageBox({
				type: "error",
				title: `${WorkbenchModeRegistry.get(this.defaultModeId).title} window failed`,
				message: "The requested Workspace could not be opened.",
				detail: message.slice(0, 8_000),
				buttons: ["OK"],
				defaultId: 0,
				cancelId: 0,
				noLink: true,
			});
		} catch (dialogError) {
			console.error("Failed to report Workbench window open failure", dialogError);
		}
	}

	private readonly onBeforeQuit = (event: ElectronEvent): void => {
		this.quitRequested = true;
		const records = this.workbenchWindows.values();
		for (const record of records) record.supervisor.dispose();
		if (this.quitAfterStateSaved || !this.persistentServices) {
			return;
		}
		event.preventDefault();
		if (this.quitSaveStarted) {
			return;
		}

		this.quitSaveStarted = true;
		for (const record of records) record.windowStateTracking.dispose();
		void (async () => {
			try {
				for (const record of records) {
					if (!record.window.isDestroyed()) await record.windowsStateHandler.saveWindowState(record.window);
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
		for (const record of this.workbenchWindows.values()) {
			record.window.webContents.send(NATIVE_HOST_ACCESSIBILITY_SUPPORT_CHANGED_CHANNEL, enabled);
		}
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
					services.userKeyboardLayout.close(),
				]).then(() => undefined)
			: Promise.resolve();
		return this.closePersistentServicesPromise;
	}

	private async resolveLocalWorkspaceTrust(supervisor: AppServerSupervisor, root: string, window?: BrowserWindow): Promise<WorkspaceTrustChoice | undefined> {
		const persisted = await readAppServerWorkspaceTrust(supervisor, root);
		if (persisted !== undefined) return WorkspaceTrustChoice.UserConfig;
		const options = {
			type: "question" as const,
			buttons: ["Trust Folder", "Open in Restricted Mode", "Cancel"],
			defaultId: 0,
			cancelId: 2,
			noLink: true,
			message: "Do you trust the authors of the files in this folder?",
			detail: "Trust enables terminals, Git mutations, tasks, and executable workspace configuration. Restricted Mode keeps file access available without running workspace code.",
		};
		const prompt = window
			? await dialog.showMessageBox(window, options)
			: await dialog.showMessageBox(options);
		if (prompt.response === 2) return undefined;
		return prompt.response === 0 ? WorkspaceTrustChoice.Trusted : WorkspaceTrustChoice.Restricted;
	}

	private appServerEnvironment(workspace: IAnyWorkspaceIdentifier): Readonly<Record<string, string>> {
		const packageLocation = {
			appPath: app.getAppPath(),
			isPackaged: app.isPackaged,
			platform: process.platform,
			resourcesPath: process.resourcesPath,
		};
		return buildAppServerEnvironment(process.env, process.platform === "win32" ? "windows" : "posix", {
			...(process.env.ZETA_RG_PATH
				? { ZETA_RG_PATH: process.env.ZETA_RG_PATH }
				: {}),
			...(process.env.ZETA_PRODUCT_SERVICES_PATH
				? { ZETA_PRODUCT_SERVICES_PATH: process.env.ZETA_PRODUCT_SERVICES_PATH }
				: {}),
			ZETA_ELECTRON_RUN_AS_NODE_PATH: process.execPath,
			ZETA_APP_SERVER_DAEMON_PATH: appServerDaemonExecutablePath(packageLocation),
			ZETA_PROFILE_ROOT: this.profileRoot,
			...(isSingleFolderWorkspaceIdentifier(workspace)
				? {
					ZETA_WORKSPACE_ROOT: workspace.uri.fsPath,
					ZETA_WORKSPACE_TRUST_SOURCE: "userConfig",
				}
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

function focusElectronWindow(window: BrowserWindow): void {
	if (window.isDestroyed()) return;
	if (window.isMinimized()) window.restore();
	window.focus();
}
