import { app } from 'electron/main';
import { join } from 'node:path';
import { developmentArtifactsPath } from '../../platform/environment/node/developmentArtifacts.js';
import type { DesktopApplicationConfiguration } from '../../product/common/product.js';
import type { WorkbenchModeId } from '../../product/common/workbenchMode.js';
import { resolveApplicationDataPaths, resolvePackagedRendererRoot } from '../../product/node/product.js';
import { type AppServerStartupMode, type ElectronMainIpcRouteContribution, ZetaApplication } from './app.js';

export interface StartElectronApplicationOptions {
	readonly application: DesktopApplicationConfiguration;
	readonly initialModeId: WorkbenchModeId;
	readonly ipcRouteContributions?: readonly ElectronMainIpcRouteContribution[];
}

/** Starts the shared Electron application with one selected initial Workbench mode. */
export function startElectronApplication(options: StartElectronApplicationOptions): void {
	const rendererBase = app.isPackaged
		? join(app.getAppPath(), 'dist', 'renderer')
		: developmentArtifactsPath(app.getAppPath(), 'renderer');
	const rendererRoot = app.isPackaged
		? resolvePackagedRendererRoot(rendererBase, options.application)
		: join(rendererBase, options.application.rendererDirectory);
	const appServerStartupMode: AppServerStartupMode = process.env.ZETA_DESKTOP_UI_ONLY === '1'
		? 'disabled'
		: 'required';

	app.setName(options.application.name);
	configureProductDataPaths(options.application);

	if (!app.requestSingleInstanceLock()) {
		app.quit();
		return;
	}

	const application = ZetaApplication.create({
		application: options.application,
		initialModeId: options.initialModeId,
		rendererRoot,
		appServerStartupMode,
		ipcRouteContributions: options.ipcRouteContributions,
	});

	app.on('second-instance', (_event, arguments_, cwd) => application.handleSecondInstance(arguments_, cwd));
	app.on('activate', () => application.handleActivate());
	app.on('window-all-closed', () => {
		if (process.platform !== 'darwin') app.quit();
	});
	app.once('ready', () => {
		void startup(application);
	});
}

async function startup(application: ZetaApplication): Promise<void> {
	try {
		await application.startupAfterReady();
	} catch (error) {
		console.error('Failed to start Zeta', error);
		await application.disposeAfterStartupFailure();
		app.exit(1);
	}
}

function configureProductDataPaths(application: DesktopApplicationConfiguration): void {
	if (process.platform === 'win32') app.setAppUserModelId(application.applicationId);
	const paths = resolveApplicationDataPaths(app.getPath('appData'), application);
	if (!hasUserDataDirectoryOverride(process.argv)) app.setPath('userData', paths.userDataPath);
	const userDataPath = app.getPath('userData');
	app.setPath('sessionData', join(userDataPath, 'session-data'));
	app.setPath('logs', join(userDataPath, 'logs'));
	app.setPath('crashDumps', join(userDataPath, 'crashes'));
}

function hasUserDataDirectoryOverride(args: readonly string[]): boolean {
	return args.some(argument => argument === '--user-data-dir' || argument.startsWith('--user-data-dir='));
}
