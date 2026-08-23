import { app } from 'electron/main';
import { join } from 'node:path';
import { ZetaDesktopApplication } from '../../product/common/product.js';
import { WorkbenchModeRegistry } from '../../product/common/workbenchMode.js';
import { readPersistedWorkbenchModeId } from '../../product/node/product.js';
import { localProfileRoot } from '../../platform/profile/node/localProfile.js';
import { debugAdapterIpcRoutes } from '../../platform/debug/electron-main/debugAdapterIpcRoutes.js';
import { startElectronApplication } from './startElectronApplication.js';

const configuredModeId = !app.isPackaged && process.env.ZETA_WORKBENCH_MODE !== undefined
	? WorkbenchModeRegistry.resolveModeId(process.env.ZETA_WORKBENCH_MODE)
	: readPersistedWorkbenchModeId(join(localProfileRoot(), 'configuration.json'), WorkbenchModeRegistry.defaultModeId);

startElectronApplication({
	application: ZetaDesktopApplication,
	initialModeId: configuredModeId,
	ipcRouteContributions: [debugAdapterIpcRoutes],
});
