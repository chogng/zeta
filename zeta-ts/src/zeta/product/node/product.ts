import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import type { DesktopApplicationConfiguration } from '../common/product.js';
import { WorkbenchModeConfigurationKey, WorkbenchModeRegistry, type WorkbenchModeId } from '../common/workbenchMode.js';

export interface DesktopApplicationDataPaths {
	readonly userDataPath: string;
	readonly sessionDataPath: string;
}

/** Resolves the persistent Electron roots shared by every built-in mode. */
export function resolveApplicationDataPaths(appDataPath: string, application: DesktopApplicationConfiguration): DesktopApplicationDataPaths {
	if (appDataPath.trim().length === 0) throw new TypeError('Application data path must not be empty');
	if (application.userDataFolderName.trim().length === 0) throw new TypeError('Application user data folder name must not be empty');
	const userDataPath = join(appDataPath, application.userDataFolderName);
	return { userDataPath, sessionDataPath: join(userDataPath, 'session-data') };
}

/** Verifies that a packaged application contains the shared Workbench and every mode-owned sibling entry. */
export function resolvePackagedRendererRoot(rendererRoot: string, application: DesktopApplicationConfiguration): string {
	const packagedRoot = join(rendererRoot, application.rendererDirectory);
	const requiredEntries = [
		join(packagedRoot, 'electron-browser', 'workbench', 'workbench.html'),
		...WorkbenchModeRegistry.definitions.flatMap(mode => mode.dedicatedSessions
			? [join(packagedRoot, 'electron-browser', 'sessions', `${mode.dedicatedSessions.rendererEntry}.html`)]
			: []),
	];
	const missing = requiredEntries.filter(entry => !existsSync(entry));
	if (missing.length > 0) throw new Error(`Packaged Zeta renderer is incomplete: ${missing.join(', ')}`);
	return packagedRoot;
}

/** Reads the preferred startup mode without making the full configuration service a bootstrap dependency. */
export function readPersistedWorkbenchModeId(configurationFilePath: string, fallback: WorkbenchModeId): WorkbenchModeId {
	let candidate: unknown;
	try {
		const document = JSON.parse(readFileSync(configurationFilePath, 'utf8')) as unknown;
		candidate = readConfigurationValue(document, WorkbenchModeConfigurationKey);
	} catch {
		return fallback;
	}
	return WorkbenchModeRegistry.isModeId(candidate) ? candidate : fallback;
}

function readConfigurationValue(document: unknown, key: string): unknown {
	if (typeof document !== 'object' || document === null || Array.isArray(document)) return undefined;
	const values = (document as { readonly values?: unknown }).values;
	if (typeof values !== 'object' || values === null || Array.isArray(values)) return undefined;
	return (values as Readonly<Record<string, unknown>>)[key];
}
